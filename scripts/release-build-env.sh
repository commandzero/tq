#!/bin/sh
# Source this before release builds. This is a source epoch, not build wall time.
# tq 0.5.0 production baseline: 21bfc9bc114c04e6739927e82d257b146eae78b3.
export SOURCE_DATE_EPOCH=1792022563

# cc forwards the epoch but does not track it. A tracked CFLAGS key invalidates
# cached allocator objects, including builds made before this policy existed.
tq_release_epoch_flag="-DTQ_RELEASE_SOURCE_DATE_EPOCH=$SOURCE_DATE_EPOCH"
case " ${CFLAGS-} " in
    *" $tq_release_epoch_flag "*) ;;
    *" -DTQ_RELEASE_SOURCE_DATE_EPOCH="*)
        echo 'Conflicting release source-epoch compiler flags; use a fresh build environment.' >&2
        return 69
        ;;
    *) CFLAGS="${CFLAGS:+$CFLAGS }$tq_release_epoch_flag" ;;
esac
export CFLAGS
unset tq_release_epoch_flag
