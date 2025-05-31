// sleep function to delay execution for a specified number of milliseconds
export const delay = (ms: number): Promise<void> =>
  new Promise((r) => setTimeout(r, ms));
