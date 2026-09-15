# Security

This is experimental compiler/runtime software. Build only trusted source packs:
Cargo dependencies and build scripts execute on the build machine. Datapacks
can issue commands with the active command source's permissions. The Fabric
runtime limits Wasm execution, memory pages and callbacks, but it is not a substitute for
server administration or reviewing packs. Metadata hashes detect changes; they
are not signatures or evidence that a pack is trustworthy.

For a vulnerability, use the repository's GitHub **Security → Report a
vulnerability** form once private vulnerability reporting is enabled. If the
form is unavailable, open an issue requesting a private contact without exploit
details or secrets. Include affected versions and a minimal private reproducer.
Do not use public issue templates to disclose credentials or exploitable details.
