import { create } from "zustand";
import { invoke } from "@tauri-apps/api/core";
import {
  Probe,
  InstallStatus,
  FirmwareUpdateResult,
  NicknameResult,
  UsbDriverMode,
  UsbDriverResult,
  DriverType,
  COMMANDS,
} from "@shared/types";

function applyDriverOverrides(
  probes: Probe[],
  overrides: Record<string, DriverType>
): Probe[] {
  return probes.map((probe) => ({
    ...probe,
    driver: overrides[probe.id] ?? probe.driver,
  }));
}

function preserveSelection(
  probes: Probe[],
  selectedProbeId: string | null
): string | null {
  if (!selectedProbeId) return null;
  return probes.some((probe) => probe.id === selectedProbeId) ? selectedProbeId : null;
}

function resetOperationStatus() {
  return {
    firmwareUpdateStatus: "idle" as const,
    firmwareUpdateMessage: "",
    nicknameStatus: "idle" as const,
    nicknameMessage: "",
    usbDriverStatus: "idle" as const,
    usbDriverMessage: "",
  };
}

interface ProbeState {
  probes: Probe[];
  driverOverrides: Record<string, DriverType>;
  isLoading: boolean;
  isInstalled: boolean | null;
  installPath: string | undefined;
  installVersion: string;
  selectedProbeId: string | null;
  error: string | null;
  firmwareUpdateStatus: "idle" | "updating" | "updated" | "current" | "failed";
  firmwareUpdateMessage: string;
  nicknameStatus: "idle" | "setting" | "success" | "failed";
  nicknameMessage: string;
  usbDriverStatus: "idle" | "switching" | "success" | "failed";
  usbDriverMessage: string;

  // Actions
  checkInstallation: () => Promise<void>;
  scanProbesSilent: () => Promise<void>;
  scanProbes: () => Promise<void>;
  selectProbe: (id: string | null) => void;
  updateFirmware: (probeIndex: number) => Promise<void>;
  setNickname: (probeIndex: number, nickname: string) => Promise<void>;
  switchUsbDriver: (probeIndex: number, mode: UsbDriverMode) => Promise<void>;
}

export const useProbeStore = create<ProbeState>((set, get) => ({
  probes: [],
  driverOverrides: {},
  isLoading: false,
  isInstalled: null,
  installPath: undefined,
  installVersion: "",
  selectedProbeId: null,
  error: null,
  firmwareUpdateStatus: "idle",
  firmwareUpdateMessage: "",
  nicknameStatus: "idle",
  nicknameMessage: "",
  usbDriverStatus: "idle",
  usbDriverMessage: "",

  // ── checkInstallation ────────────────────────────────────────────────────────
  checkInstallation: async () => {
    set({ isLoading: true, error: null });
    try {
      const result = await invoke<{
        status: InstallStatus;
        probes: Probe[];
      }>(COMMANDS.DETECT_AND_SCAN);

      const overrides = get().driverOverrides;
      const probes = applyDriverOverrides(result.probes, overrides);
      const selectedProbeId = preserveSelection(probes, get().selectedProbeId);

      set({
        isInstalled: result.status.installed,
        installPath: result.status.path,
        installVersion: result.status.version ?? "",
        probes,
        isLoading: false,
        selectedProbeId,
        ...resetOperationStatus(),
      });
    } catch (err) {
      set({
        error: err instanceof Error ? err.message : String(err),
        isLoading: false,
      });
    }
  },

  // ── scanProbes ───────────────────────────────────────────────────────────────
  scanProbes: async () => {
    set({
      isLoading: true,
      error: null,
      ...resetOperationStatus(),
    });
    try {
      const overrides = get().driverOverrides;
      const probes = applyDriverOverrides(await invoke<Probe[]>(COMMANDS.SCAN_PROBES), overrides);
      const selectedProbeId = preserveSelection(probes, get().selectedProbeId);
      set({ probes, selectedProbeId, isLoading: false });
    } catch (err) {
      set({
        error: err instanceof Error ? err.message : String(err),
        isLoading: false,
      });
    }
  },

  // ── scanProbesSilent — refresh probe list without resetting status ──────────
  scanProbesSilent: async () => {
    try {
      const overrides = get().driverOverrides;
      const probes = applyDriverOverrides(await invoke<Probe[]>(COMMANDS.SCAN_PROBES), overrides);
      const selectedProbeId = preserveSelection(probes, get().selectedProbeId);
      set({ probes, selectedProbeId });
    } catch { /* ignore */ }
  },

  // ── selectProbe ──────────────────────────────────────────────────────────────
  selectProbe: (id) => {
    const current = get().selectedProbeId;
    set({
      selectedProbeId: current === id ? null : id,
      ...resetOperationStatus(),
    });
  },

  // ── updateFirmware ───────────────────────────────────────────────────────────
  updateFirmware: async (probeIndex) => {
    set({
      firmwareUpdateStatus: "updating",
      firmwareUpdateMessage: "Updating firmware. Please wait...",
    });
    try {
      const result = await invoke<FirmwareUpdateResult>(
        COMMANDS.UPDATE_FIRMWARE,
        { probeIndex }
      );
      if (result.status === "failed") {
        set({
          firmwareUpdateStatus: "failed",
          firmwareUpdateMessage: result.error ?? "Firmware update failed.",
        });
        return;
      }
      // Update firmware field in probe list inline (avoid full re-scan)
      if (result.firmware) {
        const probes = get().probes.map((p, i) =>
          i === probeIndex ? { ...p, firmware: result.firmware } : p
        );
        set({ probes });
      }
      set({
        firmwareUpdateStatus: result.status,
        firmwareUpdateMessage: result.status === "updated"
          ? `Firmware updated successfully: ${result.firmware}`
          : `Firmware is already up to date: ${result.firmware}`,
      });
    } catch (err) {
      set({
        firmwareUpdateStatus: "failed",
        firmwareUpdateMessage: err instanceof Error ? err.message : String(err),
      });
    }
  },

  // ── setNickname ──────────────────────────────────────────────────────────────
  setNickname: async (probeIndex, nickname) => {
    set({ nicknameStatus: "setting", nicknameMessage: "" });
    try {
      const result = await invoke<NicknameResult>(COMMANDS.SET_NICKNAME, {
        probeIndex,
        nickname,
      });
      if (result.success) {
        set({
          nicknameStatus: "success",
          nicknameMessage: nickname.trim()
            ? `Nickname set to "${nickname.trim()}".`
            : "Nickname cleared.",
        });
        // Probe rebooted — re-scan after delay to reflect updated nickname
        await new Promise(resolve => setTimeout(resolve, 500));
        await get().scanProbesSilent();
      } else {
        set({
          nicknameStatus: "failed",
          nicknameMessage: result.error ?? "Could not set the nickname.",
        });
      }
    } catch (err) {
      set({
        nicknameStatus: "failed",
        nicknameMessage: err instanceof Error ? err.message : String(err),
      });
    }
  },

  // ── switchUsbDriver ──────────────────────────────────────────────────────────
  switchUsbDriver: async (probeIndex, mode) => {
    set({
      usbDriverStatus: "switching",
      usbDriverMessage: mode === "winUsb"
        ? "Switching the probe USB driver to WinUSB..."
        : "Switching the probe USB driver to SEGGER...",
      error: null,
    });

    try {
      const result = await invoke<UsbDriverResult>(COMMANDS.SWITCH_USB_DRIVER, { probeIndex, mode });

      if (!result.success) {
        set({
          usbDriverStatus: "failed",
          usbDriverMessage: result.error ?? "Could not switch the USB driver.",
        });
        return;
      }

      const probes = get().probes;
      const probe = probes[probeIndex];
      if (probe) {
        const newDriver: DriverType = mode === "winUsb" ? "WinUSB" : "SEGGER";
        set({
          driverOverrides: { ...get().driverOverrides, [probe.id]: newDriver },
          probes: probes.map((p, i) => (i === probeIndex ? { ...p, driver: newDriver } : p)),
        });
      }

      // Give the probe a brief moment to detach/reattach after reboot.
      await new Promise((resolve) => setTimeout(resolve, 900));
      await get().scanProbesSilent();

      set({
        usbDriverStatus: "success",
        usbDriverMessage: mode === "winUsb"
          ? "Switched to WinUSB. The probe may reboot briefly. You may need to unplug and replug your probe to apply the configuration changes."
          : "Switched to SEGGER. The probe may reboot briefly. You may need to unplug and replug your probe to apply the configuration changes.",
      });
    } catch (err) {
      set({
        usbDriverStatus: "failed",
        usbDriverMessage: err instanceof Error ? err.message : String(err),
      });
    }
  },
}));