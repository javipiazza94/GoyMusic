import { createContext, useContext } from "react";

/** The scrolling `<main>`: long lists virtualize against it so the whole view scrolls as one. */
export const ScrollParent = createContext<HTMLElement | null>(null);

export const useScrollParent = () => useContext(ScrollParent);
