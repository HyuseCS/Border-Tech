/*++

Lampyris DebugView tracing.

Every [LAMPYRIS] trace in the driver goes through LampyrisTrace. Set
LAMPYRIS_TRACE to 1 (here, or as a preprocessor define on the build) to get the
DebugView output back - it is what diagnosed the capture-pin, format-negotiation
and stream-lifecycle bugs on 2026-08-21.

Disabled, the macro still compiles its arguments inside `if (0)`, so nothing is
emitted but variables computed purely for tracing do not become unused - which
would be an error under /W4 /WX.

--*/

#ifndef _LAMPYRIS_DEBUG_H_
#define _LAMPYRIS_DEBUG_H_

#ifndef LAMPYRIS_TRACE
#define LAMPYRIS_TRACE 0
#endif

#if LAMPYRIS_TRACE
#define LampyrisTrace(...) DbgPrint(__VA_ARGS__)
#else
#define LampyrisTrace(...) do { if (0) DbgPrint(__VA_ARGS__); } while (0)
#endif

#endif // _LAMPYRIS_DEBUG_H_
