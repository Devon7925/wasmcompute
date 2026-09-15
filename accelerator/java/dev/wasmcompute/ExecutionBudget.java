package dev.wasmcompute;

/** Per-server-thread budget shared by nested modules, without timer threads. */
public final class ExecutionBudget {
    private static final ThreadLocal<State> CURRENT=ThreadLocal.withInitial(State::new);
    private static final class State {int depth;long remaining;}
    private ExecutionBudget(){}
    public static void enter(){var state=CURRENT.get();if(state.depth==0)state.remaining=100_000_000L;state.depth++;}
    public static void leave(){var state=CURRENT.get();if(--state.depth==0)CURRENT.remove();}
    public static void check(){
        var state=CURRENT.get();
        if(state.depth==0)throw new IllegalStateException("Guest executed without an execution budget");
        if(--state.remaining<0||Thread.currentThread().isInterrupted())throw new RuntimeOps.Trap(5);
    }
}
