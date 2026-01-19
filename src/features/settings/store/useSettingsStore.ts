import { create } from 'zustand';
import { invoke } from '@tauri-apps/api/core';

export interface GameSettings {
    ram_min_gb: number;
    ram_max_gb: number;
    custom_java_args: string;
    close_on_launch: boolean;
    minimize_to_tray: boolean;
    discord_rpc_enabled: boolean;
}

interface SettingsState {
    settings: GameSettings;
    loading: boolean;
    loadSettings: () => Promise<void>;
    updateSettings: (settings: Partial<GameSettings>) => Promise<void>;
}

export const useSettingsStore = create<SettingsState>((set, get) => ({
    settings: {
        ram_min_gb: 2,
        ram_max_gb: 4,
        custom_java_args: '',
        close_on_launch: false,
        minimize_to_tray: true,
        discord_rpc_enabled: true,
    },
    loading: false,

    loadSettings: async () => {
        set({ loading: true });
        try {
            const settings = await invoke<GameSettings>('get_game_settings');
            set({ settings });
        } catch (error) {
            console.error('Failed to load settings:', error);
        } finally {
            set({ loading: false });
        }
    },

    updateSettings: async (newSettings) => {
        const updated = { ...get().settings, ...newSettings };
        set({ settings: updated });
        try {
            await invoke('set_game_settings', { settings: updated });
        } catch (error) {
            console.error('Failed to save settings:', error);
        }
    },
}));
