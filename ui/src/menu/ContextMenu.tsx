import { useEffect, useLayoutEffect, useRef, useState, type KeyboardEvent } from "react";
import { useMenu, type MenuItem } from "./menu-store";
import styles from "./ContextMenu.module.css";

function MenuList({
  items,
  x,
  y,
  label,
  onClose,
  onBack,
}: {
  items: MenuItem[];
  x: number;
  y: number;
  label: string;
  onClose(restoreFocus: boolean): void;
  onBack?(): void;
}) {
  const ref = useRef<HTMLDivElement>(null);
  const [pos, setPos] = useState({ x, y });
  const [sub, setSub] = useState<{ index: number; x: number; y: number } | null>(null);

  // Keep the menu inside the window.
  useLayoutEffect(() => {
    const el = ref.current;
    if (!el) return;
    const { width, height } = el.getBoundingClientRect();
    setPos({ x: Math.max(4, Math.min(x, window.innerWidth - width - 4)), y: Math.max(4, Math.min(y, window.innerHeight - height - 4)) });
  }, [x, y]);

  useEffect(() => {
    ref.current?.querySelector<HTMLElement>('[role="menuitem"]')?.focus();
  }, []);

  const buttons = () => Array.from(ref.current?.querySelectorAll<HTMLElement>(':scope > [role="menuitem"]') ?? []);

  function openSub(index: number, el: HTMLElement) {
    const r = el.getBoundingClientRect();
    setSub({ index, x: r.right - 4, y: r.top - 4 });
  }

  function activate(item: MenuItem, index: number, el: HTMLElement) {
    if (item.disabled) return;
    if (item.submenu) return openSub(index, el);
    onClose(true);
    item.onSelect?.();
  }

  function onKeyDown(e: KeyboardEvent) {
    const list = buttons();
    const at = list.indexOf(document.activeElement as HTMLElement);
    const focus = (i: number) => list[(i + list.length) % list.length]?.focus();
    switch (e.key) {
      case "ArrowDown":
        focus(at + 1);
        break;
      case "ArrowUp":
        focus(at - 1);
        break;
      case "Home":
        focus(0);
        break;
      case "End":
        focus(list.length - 1);
        break;
      case "ArrowRight":
        if (at >= 0 && items[at].submenu && !items[at].disabled) openSub(at, list[at]);
        break;
      case "ArrowLeft":
        if (onBack) onBack();
        break;
      case "Escape":
        onClose(true);
        break;
      case "Tab":
        onClose(true);
        break;
      default:
        return;
    }
    e.preventDefault();
    e.stopPropagation();
  }

  return (
    <div
      ref={ref}
      role="menu"
      aria-label={label}
      className={styles.menu}
      style={{ left: pos.x, top: pos.y }}
      onKeyDown={onKeyDown}
      onContextMenu={(e) => e.preventDefault()}
    >
      {items.map((item, index) => (
        <button
          key={item.label}
          type="button"
          role="menuitem"
          tabIndex={-1}
          aria-disabled={item.disabled || undefined}
          aria-haspopup={item.submenu ? "menu" : undefined}
          aria-expanded={item.submenu ? sub?.index === index : undefined}
          data-danger={item.danger || undefined}
          className={styles.item}
          onClick={(e) => activate(item, index, e.currentTarget)}
          onMouseEnter={(e) => {
            if (item.submenu && !item.disabled) openSub(index, e.currentTarget);
            else setSub(null);
          }}
        >
          <span>{item.label}</span>
          {item.submenu && <span aria-hidden="true">›</span>}
        </button>
      ))}
      {sub && items[sub.index]?.submenu && (
        <MenuList
          items={items[sub.index].submenu!}
          x={sub.x}
          y={sub.y}
          label={items[sub.index].label}
          onClose={onClose}
          onBack={() => {
            const index = sub.index;
            setSub(null);
            buttons()[index]?.focus();
          }}
        />
      )}
    </div>
  );
}

/** The single context menu of the window (spec §7); opened through `useMenu().show`. */
export function ContextMenu() {
  const open = useMenu((s) => s.open);
  const close = useMenu((s) => s.close);
  const root = useRef<HTMLDivElement>(null);

  useEffect(() => {
    if (!open) return;
    const onPointerDown = (e: PointerEvent) => {
      if (!root.current?.contains(e.target as Node)) close();
    };
    const onBlur = () => close();
    document.addEventListener("pointerdown", onPointerDown);
    window.addEventListener("blur", onBlur);
    window.addEventListener("resize", onBlur);
    return () => {
      document.removeEventListener("pointerdown", onPointerDown);
      window.removeEventListener("blur", onBlur);
      window.removeEventListener("resize", onBlur);
    };
  }, [open, close]);

  if (!open) return null;
  return (
    <div ref={root}>
      <MenuList
        key={`${open.x},${open.y}`}
        items={open.items}
        x={open.x}
        y={open.y}
        label="Acciones"
        onClose={(restoreFocus) => {
          close();
          if (restoreFocus) open.returnFocus?.focus();
        }}
      />
    </div>
  );
}
