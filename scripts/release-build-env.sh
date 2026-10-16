#!/bin/sh
# Source this before release builds. This is a source epoch, not build wall time.
# tq 0.5.0 production baseline: 21bfc9bc114c04e6739927e82d257b146eae78b3.
export SOURCE_DATE_EPOCH=1792022563

# cc forwards the epoch but does not track it. A tracked CFLAGS key invalidates
# cached allocator objects, including builds made before this policy existed.
tq_release_epoch_flag="-DTQ_RELEASE_SOURCE_DATE_EPOCH=$SOURCE_DATE_EPOCH"
tq_release_remaining_flags=" ${CFLAGS-} "
tq_release_has_epoch=false
case "$tq_release_remaining_flags" in
    *" $tq_release_epoch_flag "*)
        # Remove exactly one canonical token before rejecting any other key.
        tq_release_epoch_suffix=${tq_release_remaining_flags#*" $tq_release_epoch_flag "}
        tq_release_remaining_flags="${tq_release_remaining_flags%%" $tq_release_epoch_flag "*} $tq_release_epoch_suffix"
        tq_release_has_epoch=true
        ;;
esac
case "$tq_release_remaining_flags" in
    *TQ_RELEASE_SOURCE_DATE_EPOCH*)
        echo 'Conflicting release source-epoch compiler flags; use a fresh build environment.' >&2
        return 69
        ;;
esac
if [ "$tq_release_has_epoch" = false ]; then
    CFLAGS="${CFLAGS:+$CFLAGS }$tq_release_epoch_flag"
fi
export CFLAGS
unset tq_release_epoch_flag tq_release_remaining_flags tq_release_has_epoch tq_release_epoch_suffix
