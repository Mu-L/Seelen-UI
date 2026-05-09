import type { TwmRuntimeNode, TwmRuntimeTree } from "@seelen-ui/lib/types";

export { TwmNodeKind } from "@seelen-ui/lib/types";

export enum Reservation {
  Left = "Left",
  Right = "Right",
  Top = "Top",
  Bottom = "Bottom",
  Stack = "Stack",
  Float = "Float",
}

export enum Sizing {
  Increase = "Increase",
  Decrease = "Decrease",
}

export type Node = TwmRuntimeNode;
export type Tree = TwmRuntimeTree;

export const TREE_CONTEXT_KEY = "wm-tree";
