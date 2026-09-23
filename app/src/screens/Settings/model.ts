import type { MessageDescriptor } from "@lingui/core";
import { msg } from "@lingui/core/macro";
import {
  ArrowsClockwiseIcon,
  BankIcon,
  BriefcaseIcon,
  CloudArrowDownIcon,
  DatabaseIcon,
  PaletteIcon,
  PuzzlePieceIcon,
  SparkleIcon,
  TagIcon,
  UsersIcon,
  type Icon,
} from "@phosphor-icons/react";
import type { Portfolio, PortfolioInput } from "../../lib/types";

/** The categories of the settings rail, in the order they are offered. */
export type CategoryId =
  | "portfolio"
  | "accounts"
  | "attributes"
  | "market"
  | "ai"
  | "appearance"
  | "plugins"
  | "profiles"
  | "data"
  | "updates";

export const CATEGORIES: { id: CategoryId; label: MessageDescriptor; icon: Icon }[] = [
  { id: "portfolio", label: msg`Portfolio`, icon: BriefcaseIcon },
  { id: "accounts", label: msg`Accounts`, icon: BankIcon },
  { id: "attributes", label: msg`Attributes`, icon: TagIcon },
  { id: "market", label: msg`Market data`, icon: CloudArrowDownIcon },
  { id: "ai", label: msg`AI assistant`, icon: SparkleIcon },
  { id: "appearance", label: msg`Appearance`, icon: PaletteIcon },
  { id: "plugins", label: msg`Plugins`, icon: PuzzlePieceIcon },
  { id: "profiles", label: msg`Profiles`, icon: UsersIcon },
  { id: "data", label: msg`Data and storage`, icon: DatabaseIcon },
  { id: "updates", label: msg`Updates`, icon: ArrowsClockwiseIcon },
];

/**
 * `portfolio_save` takes the whole portfolio, so a panel editing one part of it
 * starts from everything that is stored and replaces only its own fields.
 */
export function inputOf(portfolio: Portfolio): PortfolioInput {
  return {
    name: portfolio.name,
    base_currency: portfolio.base_currency,
    cost_basis_method: portfolio.cost_basis_method,
    account_ids: portfolio.account_ids,
  };
}
