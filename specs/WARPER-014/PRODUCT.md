# WARPER-014: stable alternate-screen background

## Summary

Warper should keep alternate-screen application content colors inside the cells that set them. A dominant highlight, diff, selection, or diagnostic color must not replace the background of the entire terminal surface or adjacent Warper-owned UI. Warper may extend an application-defined background beyond the grid when the sampled screen is sufficiently uniform to identify that color as the application's canvas.

## Problem

Warper samples alternate-screen cell backgrounds and uses the most frequent color as the background beneath the complete alternate-screen element. This works for applications that paint a uniform custom canvas, but it also treats any large content region as the canvas.

In a side-by-side diff viewer, the diff pane is wider than the file pane. A screen containing mostly removed or added lines therefore makes the removal or addition background the most frequent sampled color. Warper paints that color beneath the entire alternate-screen element. Transparent cells in the file pane, borders, terminal padding, and Warper-owned agent controls expose the inferred color even though the application scoped it to the diff rows.

## Behavior

1. Explicit terminal cell backgrounds remain unchanged.
2. A content color that covers a simple majority of sampled cells is not sufficient to identify the alternate-screen canvas.
3. Warper infers a solid alternate-screen canvas only when one opaque color accounts for at least 90% of recorded samples.
4. Transparent/default-background cells participate in the confidence calculation and cannot be discarded when deciding whether an opaque color is uniform.
5. When no color meets the confidence requirement, the alternate-screen surface uses the configured Warper theme background.
6. Warper-owned UI that surrounds an alternate-screen CLI agent uses the inferred color only when it meets the same confidence requirement. Otherwise it retains its normal theme background.
7. Alternate-screen applications that paint a nearly uniform custom background continue to blend into terminal padding and supported Warper-owned controls.
8. The decision is deterministic when sampled colors tie or compete. No opaque color is inferred unless it independently meets the confidence requirement.
9. Leaving the alternate screen or resetting the sampler does not retain a prior inferred color.

## Validation

- A sampler containing one opaque color returns that color.
- A sampler with at least 90% of its recorded samples in one opaque color returns that color.
- A sampler with less than 90% of its recorded samples in one opaque color returns no inferred color.
- Transparent samples count toward the total and prevent a dominant content color from being treated as uniform.
- Competing opaque colors return no inferred color.
- A transparent winner returns no inferred color.
- Resetting the sampler clears its samples and inferred color.
- The alternate-screen renderer falls back to the configured Warper theme background when inference returns no color.
- Existing alternate-screen rendering and CLI-agent footer tests pass.
