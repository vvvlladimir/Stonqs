/** Wording shared by the marks that name a bucket: the trail, the treemap and the sunburst. */

/** Places the ticker before the optional full security name. */
export function fullLabel(short: string, full?: string): string {
  return full && full !== short ? `${short} — ${full}` : short;
}
