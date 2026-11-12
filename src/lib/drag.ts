/** Convert the native drag position to CSS pixels for DOM hit testing.
 * Wry 0.57 uses AppKit points on macOS and GTK logical coordinates on Linux,
 * despite Tauri typing these as PhysicalPosition. Windows uses client pixels.
 */
export function dragClientPosition(position: { x: number; y: number }, platform: string, pixelRatio: number) {
  const scale = platform === 'windows' && Number.isFinite(pixelRatio) && pixelRatio > 0 ? pixelRatio : 1;
  return { x: position.x / scale, y: position.y / scale };
}
