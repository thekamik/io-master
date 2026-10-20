// The program and the account (serializer/app_view.rs; tauri_ui/commands/account.rs).
//
// The types mirror the views of src/tauri_ui/serializer/; a change
// there needs the same change here.

import { invoke } from "@tauri-apps/api/core";

export interface PageView {
  id: string;
  title: string;
}

export interface WarningView {
  level: "danger" | "warning";
  text: string;
}

export interface AppView {
  installed: boolean;
  role: "workstation" | "master" | null;
  user: string | null;
  pages: PageView[];
  warnings: WarningView[];
  /** The version of the program (`Cargo.toml`): "0.1.0". */
  version: string;
  /** The folder of the installation. */
  data_dir: string;
}

export function appState(): Promise<AppView> {
  return invoke("app_state");
}

export function setUp(fields: {
  name: string;
  password: string;
  passwordRepeat: string;
  role: string;
  roleConfirmed: boolean;
}): Promise<AppView> {
  return invoke("set_up", fields);
}

export function logIn(name: string, password: string): Promise<AppView> {
  return invoke("log_in", { name, password });
}

export function logOut(): Promise<AppView> {
  return invoke("log_out");
}

export function changePassword(current: string, password: string, passwordRepeat: string): Promise<void> {
  return invoke("change_password", { current, password, passwordRepeat });
}
