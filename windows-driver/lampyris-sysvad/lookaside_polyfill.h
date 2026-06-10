#pragma once

#include <wdm.h>

FORCEINLINE
PVOID
LampyrisExAllocateFromNPagedLookasideList(
    _Inout_ PNPAGED_LOOKASIDE_LIST Lookaside
    )
{
    PVOID Entry;
    Lookaside->L.TotalAllocates += 1;
    Entry = InterlockedPopEntrySList(&Lookaside->L.ListHead);
    if (Entry == NULL) {
        Lookaside->L.AllocateMisses += 1;
        Entry = (Lookaside->L.Allocate)(Lookaside->L.Type,
                                        Lookaside->L.Size,
                                        Lookaside->L.Tag);
    }
    return Entry;
}

FORCEINLINE
VOID
LampyrisExFreeToNPagedLookasideList(
    _Inout_ PNPAGED_LOOKASIDE_LIST Lookaside,
    _In_ PVOID Entry
    )
{
    Lookaside->L.TotalFrees += 1;
    if (ExQueryDepthSList(&Lookaside->L.ListHead) >= Lookaside->L.Depth) {
        Lookaside->L.FreeMisses += 1;
        (Lookaside->L.Free)(Entry);
    } else {
        InterlockedPushEntrySList(&Lookaside->L.ListHead, (PSLIST_ENTRY)Entry);
    }
}

#undef ExAllocateFromNPagedLookasideList
#define ExAllocateFromNPagedLookasideList LampyrisExAllocateFromNPagedLookasideList

#undef ExFreeToNPagedLookasideList
#define ExFreeToNPagedLookasideList LampyrisExFreeToNPagedLookasideList
