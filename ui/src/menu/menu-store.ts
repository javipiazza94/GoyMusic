import { create } from "zustand";

export type MenuItem = {
  label: string;
  onSelect?(): void;
  disabled?: boolean;
  submenu?: MenuItem[];
  danger?: boolean;
};

type OpenMenu = { x: number; y: number; items: MenuItem[]; returnFocus: HTMLElement | null };

export const useMenu = create<{
  open: OpenMenu | null;
  show(x: number, y: number, items: MenuItem[], returnFocus?: HTMLElement | null): void;
  close(): void;
}>((set) => ({
  open: null,
  show: (x, y, items, returnFocus = null) => set({ open: { x, y, items, returnFocus } }),
  close: () => set({ open: null }),
}));
