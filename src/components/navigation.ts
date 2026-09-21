import {
  Box,
  Folder,
  Home,
  ScrollText,
  Settings,
  Server,
  Braces,
  type LucideIcon,
} from "lucide-react";
import { Page, type Page as AppPage } from "../domain";

export type NavEntry = { id: AppPage; title: string; icon: LucideIcon };
export const workspaceNav: NavEntry[] = [
  { id: Page.Overview, title: "Genel bakış", icon: Home },
  { id: Page.Projects, title: "Projeler", icon: Folder },
  { id: Page.Logs, title: "Günlükler", icon: ScrollText },
];
export const environmentNav: NavEntry[] = [
  { id: Page.Packages, title: "Bileşenler", icon: Box },
  { id: Page.Services, title: "Hizmetler", icon: Server },
];
export const manageNav: NavEntry[] = [
  { id: Page.Api, title: "API ve MCP", icon: Braces },
  { id: Page.Settings, title: "Ayarlar", icon: Settings },
];
export const navigation = [...workspaceNav, ...environmentNav, ...manageNav];
export const pageLabel = (page: AppPage) =>
  navigation.find((item) => item.id === page)?.title ?? "Sayfa";
