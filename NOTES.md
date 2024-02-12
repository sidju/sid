# Language design concepts

## Parallel

The sid language works by constructing the abstract syntax tree, optimises away
what it can, splits it into segments that take and return small amounts of data,
and then runs those segments in parallel as much as possible.
(This will be limited by dynamic data and may turn out to be impossible. If so
another automatic parallelization strategy will be devised.)

It is, as of yet, unspecified if these will run as batches (running a large nr.
of segments until they reach a synchronization point from which the next batch
of segments construct their state and run) or dynamically.

This will require some manner of green threads when running on x86 with OS based
threading, but the idea is that it should be able to compile to run directly on
GPUs or similar hardware.
