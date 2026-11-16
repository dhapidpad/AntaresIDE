# Fake agent fixtures

Future P1 process tests should launch a small deterministic child executable or test helper from this directory. It must support controlled stdout frames, stderr output, delayed replies, malformed frames, abrupt exit, and cancellation scenarios. The helper must never make a network request or invoke a real model.
