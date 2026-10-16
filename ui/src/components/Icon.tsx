/** Hand-drawn line icons on a 24 px grid; they inherit the text colour. */
export type IconName =
  | "play" | "pause" | "next" | "previous" | "shuffle" | "repeat" | "repeatOne" | "heart" | "heartFilled"
  | "queue" | "volume" | "volumeMuted" | "search" | "home" | "settings" | "plus" | "folder" | "more" | "close"
  | "music";

const paths: Record<IconName, string> = {
  play: "M7 4.5v15l12-7.5z",
  pause: "M7 4.5h3.5v15H7zM13.5 4.5H17v15h-3.5z",
  next: "M5 5l10 7-10 7zM18 5v14",
  previous: "M19 5L9 12l10 7zM6 5v14",
  shuffle: "M3 7h3.5c4 0 6 10 10.5 10H21M3 17h3.5c1.6 0 2.8-1.6 3.9-3.6M13.6 9.6C14.7 8 15.6 7 17 7H21M18 4l3 3-3 3M18 14l3 3-3 3",
  repeat: "M4 11V9a2 2 0 0 1 2-2h13M16 4l3 3-3 3M20 13v2a2 2 0 0 1-2 2H5M8 20l-3-3 3-3",
  repeatOne: "M4 11V9a2 2 0 0 1 2-2h13M16 4l3 3-3 3M20 13v2a2 2 0 0 1-2 2H5M8 20l-3-3 3-3M11 10.5l1.5-1v5",
  heart: "M12 20s-7.5-4.4-7.5-10A4.3 4.3 0 0 1 12 7.4 4.3 4.3 0 0 1 19.5 10C19.5 15.6 12 20 12 20z",
  heartFilled: "M12 20s-7.5-4.4-7.5-10A4.3 4.3 0 0 1 12 7.4 4.3 4.3 0 0 1 19.5 10C19.5 15.6 12 20 12 20z",
  queue: "M4 6h12M4 11h12M4 16h7M17 14v6l4-3z",
  volume: "M4 9.5h3.5L12 5.5v13l-4.5-4H4zM15.5 9a4 4 0 0 1 0 6M18 6.5a7.5 7.5 0 0 1 0 11",
  volumeMuted: "M4 9.5h3.5L12 5.5v13l-4.5-4H4zM16 9.5l5 5M21 9.5l-5 5",
  search: "M10.5 17a6.5 6.5 0 1 0 0-13 6.5 6.5 0 0 0 0 13zM15.5 15.5L20 20",
  home: "M4 10.5L12 4l8 6.5V20h-5.5v-5.5h-5V20H4z",
  settings: "M12 15a3 3 0 1 0 0-6 3 3 0 0 0 0 6zM12 3v2.5M12 18.5V21M3 12h2.5M18.5 12H21M5.6 5.6l1.8 1.8M16.6 16.6l1.8 1.8M5.6 18.4l1.8-1.8M16.6 7.4l1.8-1.8",
  plus: "M12 5v14M5 12h14",
  folder: "M3.5 6.5a1.5 1.5 0 0 1 1.5-1.5h4l2 2.5h8a1.5 1.5 0 0 1 1.5 1.5v8.5a1.5 1.5 0 0 1-1.5 1.5H5a1.5 1.5 0 0 1-1.5-1.5z",
  more: "M6 12h.01M12 12h.01M18 12h.01",
  close: "M6 6l12 12M18 6L6 18",
  music: "M9 17.5V6l10-2v11.5M9 17.5a2.5 2.5 0 1 1-5 0 2.5 2.5 0 0 1 5 0zM19 15.5a2.5 2.5 0 1 1-5 0 2.5 2.5 0 0 1 5 0z",
};

const filled = new Set<IconName>(["play", "pause", "heartFilled"]);

export function Icon({ name, size = 20 }: { name: IconName; size?: number }) {
  const solid = filled.has(name);
  return (
    <svg
      width={size}
      height={size}
      viewBox="0 0 24 24"
      fill={solid ? "currentColor" : "none"}
      stroke="currentColor"
      strokeWidth={name === "more" ? 3 : 1.75}
      strokeLinecap="round"
      strokeLinejoin="round"
      aria-hidden="true"
      focusable="false"
    >
      <path d={paths[name]} />
    </svg>
  );
}
