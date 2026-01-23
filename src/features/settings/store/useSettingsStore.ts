import { create } from 'zustand';
import { invoke } from '@tauri-apps/api/core';
import i18n from '../../../i18n';

export interface GameSettings {
    ram_gb: number;
    custom_java_args: string;
    close_on_launch: boolean;
    minimize_to_tray: boolean;
    discord_rpc_enabled: boolean;
    channel: string;
    language: string;
    active_version: number;
    player_name: string;
    override_os: string | null;
    override_arch: string | null;
}

interface SettingsState {
    settings: GameSettings;
    loading: boolean;
    loadSettings: () => Promise<void>;
    updateSettings: (settings: Partial<GameSettings>) => Promise<void>;
}

const applyLanguage = (lang: string) => {
    if (lang === 'auto') {
        const systemLang = navigator.language.split('-')[0];
        i18n.changeLanguage(systemLang);
    } else {
        i18n.changeLanguage(lang);
    }
};

export const useSettingsStore = create<SettingsState>((set, get) => ({
    settings: {
        ram_gb: 4,
        custom_java_args: '',
        close_on_launch: false,
        minimize_to_tray: true,
        discord_rpc_enabled: true,
        channel: 'release',
        language: 'auto',
        active_version: 0,
        player_name: 'Player',
        override_os: null,
        override_arch: null,
    },
    loading: false,

    loadSettings: async () => {
        set({ loading: true });
        try {
            const settings = await invoke<GameSettings>('get_game_settings');
            set({ settings });
            applyLanguage(settings.language);
        } catch (error) {
            console.error('Failed to load settings:', error);
        } finally {
            set({ loading: false });
        }
    },

    updateSettings: async (newSettings) => {
        const previousSettings = get().settings;
        const updated = { ...previousSettings, ...newSettings };

        set({ settings: updated, loading: true }); // Keep loading false? or distinct saving state? existing code didn't use loading here much but let's leave it simple

        if (newSettings.language) {
            applyLanguage(newSettings.language);
        }

        try {
            await invoke('set_game_settings', { settings: updated });
            set({ loading: false });
        } catch (error) {
            console.error('Failed to save settings:', error);
            // Rollback
            set({ settings: previousSettings, loading: false });
            if (newSettings.language) {
                applyLanguage(previousSettings.language);
            }
            // Optional: You could expose an error state here if the interface supports it
        }
    },
}));
