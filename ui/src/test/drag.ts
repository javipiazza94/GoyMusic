/** Minimal DataTransfer for jsdom, which has none. */
export function fakeDataTransfer(initial: Record<string, string> = {}) {
  const data: Record<string, string> = { ...initial };
  return {
    data,
    dropEffect: "none",
    effectAllowed: "all",
    setData: (type: string, value: string) => void (data[type] = value),
    getData: (type: string) => data[type] ?? "",
    get types() {
      return Object.keys(data);
    },
    setDragImage: () => {},
  };
}
