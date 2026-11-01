import { People, Settings, ViewList } from "@mui/icons-material";

import type { PageInfo } from "../components/SideMenu";

export const pageList: PageInfo[] = [
  { label: "节点", path: "/", icon: <ViewList fontSize="small" /> },
  { label: "用户", path: "/users", icon: <People fontSize="small" /> },
  { label: "设置", path: "/settings", icon: <Settings fontSize="small" /> },
];
