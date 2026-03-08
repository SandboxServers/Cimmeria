export function clampProgressValue(value: number) {
  return Math.min(100, Math.max(0, value));
}

export function getPlayerStatusVariant(status: string) {
  switch (status) {
    case 'Combat':
      return 'destructive';
    case 'In mission':
      return 'default';
    case 'Crafting':
      return 'secondary';
    default:
      return 'success';
  }
}
