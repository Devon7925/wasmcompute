import dev.wasmcompute.WasmRuntime;
import com.google.gson.*;
import java.nio.file.*;
import java.util.*;

/** Checks the shipped, shaded jar without Minecraft, JNI or platform libraries. */
public final class RuntimeTest {
    private static int passed;
    private static void check(boolean ok, String label) {
        if (!ok) throw new AssertionError(label);
        passed++;
    }
    private static void result(long id, String name, long... expected) {
        long[] actual = WasmRuntime.invoke(id, name, new long[0]);
        check(Arrays.equals(actual, expected), name + ": " + Arrays.toString(actual));
    }
    public static void main(String[] args) throws Exception {
        Path fixtures = Path.of(args[0]);
        long id = WasmRuntime.create(text -> (1L << 32) | 1, Files.readAllBytes(fixtures.resolve("numeric.wasm")));
        var cases = JsonParser.parseString(Files.readString(fixtures.resolve("numeric-cases.json"))).getAsJsonArray();
        for (var item : cases) {
            var row = item.getAsJsonObject();
            var values = row.getAsJsonArray("args");
            long[] inputs = new long[values.size()];
            for (int i = 0; i < inputs.length; i++) inputs[i] = values.get(i).getAsBigInteger().longValue();
            long[] actual = WasmRuntime.invoke(id, row.get("name").getAsString(), inputs);
            int trap = row.get("trap").getAsInt();
            check(actual[0] == trap && actual[1] == 0 && (trap != 0 ||
                (actual.length == 3 && actual[2] == row.get("result").getAsBigInteger().longValue())), row + " => " + Arrays.toString(actual));
        }
        WasmRuntime.close(id);
        byte[] guards = Files.readAllBytes(fixtures.resolve("guards.wasm"));
        long[] handle = new long[1];
        int[] calls = new int[1];
        handle[0] = WasmRuntime.create(text -> {
            check(text.equals("probe"), "host command text");
            calls[0]++;
            result(handle[0], "ok", 0, 0, 42); // Same-instance reentry.
            return (1L << 32) | 321;
        }, guards);
        id = handle[0];
        result(id, "command", 0, 0, 321);
        check(calls[0] == 1, "exactly once host effects");
        for (String name : List.of("bad_utf8", "bad_control", "bad_bounds", "bad_length")) result(id, name, 0, 1, 0);
        result(id, "ok", 0, 0, 42); // A host error does not poison the next call.
        result(id, "stack_trap", 1, 0);
        result(id, "stack", 0, 0, 1024);
        check(Arrays.equals(WasmRuntime.invoke(id, "grow", new long[]{1}), new long[]{0, 0, 1}), "memory growth");
        check(Arrays.equals(WasmRuntime.invoke(id, "grow", new long[]{1}), new long[]{0, 0, -1}), "declared memory maximum");
        for (String name : List.of("loop", "loop_if", "loop_table", "recursion")) {
            result(id, name, 5, 0);
            result(id, "ok", 0, 0, 42);
        }
        Thread.currentThread().interrupt();
        try { result(id, "loop", 5, 0); }
        finally { Thread.interrupted(); }
        result(id, "ok", 0, 0, 42);
        WasmRuntime.close(id);
        try { WasmRuntime.invoke(id, "ok", new long[0]); throw new AssertionError("closed instance accepted"); }
        catch (IllegalStateException expected) { passed++; }
        System.out.println("Runtime checks passed: " + passed + " (" + cases.size() + " numeric cases); " + System.getProperty("os.name"));
    }
}
