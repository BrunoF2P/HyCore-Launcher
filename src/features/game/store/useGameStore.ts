import { create } from 'zustand';
import { invoke } from '@tauri-apps/api/core';
import i18next from '../../../i18n';

type ButtonState = 'idle' | 'checking' | 'ready' | 'update_available' | 'updating' | 'launching';

interface GameState {
    buttonState: ButtonState;
    updateAvailable: boolean;
    latestVersion: number;
    installedVersions: any[];
    activeVersion: number;
    lastError: string | null;
    setButtonState: (state: ButtonState) => void;
    checkForUpdates: () => Promise<void>;
    fetchLocalVersions: () => Promise<void>;
    switchVersion: (version: number) => Promise<void>;
    launchGame: () => Promise<void>;
    clearError: () => void;
}

export const useGameStore = create<GameState>((set, get) => ({
    buttonState: 'idle',
    updateAvailable: false,
    latestVersion: 0,
    installedVersions: [],
    activeVersion: 0,
    lastError: null,

    setButtonState: (state) => set({ buttonState: state }),
    clearError: () => set({ lastError: null }),

    fetchLocalVersions: async () => {
        try {
            const manifest = await invoke<any>('get_local_manifest_command');
            set({
                installedVersions: manifest.installed,
                activeVersion: manifest.active_version
            });
        } catch (err) {
            console.error('Failed to fetch local versions:', err);
            set({ lastError: i18next.t('error.local_db') });
        }
    },

    switchVersion: async (version: number) => {
        try {
            await invoke('switch_version_command', { version });
            await get().fetchLocalVersions();
        } catch (err) {
            console.error('Failed to switch version:', err);
            set({ lastError: i18next.t('error.switch_version') });
        }
    },

    checkForUpdates: async () => {
        set({ buttonState: 'checking', lastError: null });
        await get().fetchLocalVersions();

        try {
            const [updateAvailable, latestVersion] = await invoke<[boolean, number]>('check_for_game_update');

            set({
                updateAvailable,
                latestVersion,
                buttonState: updateAvailable ? 'update_available' : 'ready',
            });
        } catch (err) {
            console.error('Failed to check for updates:', err);
            set({
                buttonState: 'ready',
                lastError: i18next.t('error.check_update')
            });
        }
    },

    launchGame: async () => {
        set({ buttonState: 'launching', lastError: null });

        try {
            await invoke('launch_game');
            setTimeout(() => {
                set({ buttonState: 'ready' });
            }, 2000);
        } catch (err: any) {
            console.error('Failed to launch game:', err);
            const errorMsg = String(err);
            set({ lastError: errorMsg });

            // Only trigger update check if it looks like a missing game
            if (errorMsg.includes("Game not installed")) {
                set({ buttonState: 'checking' });
                try {
                    const [, latestVersion] = await invoke<[boolean, number]>('check_for_game_update');
                    set({
                        updateAvailable: true,
                        latestVersion,
                        buttonState: 'update_available',
                    });
                } catch (e) {
                    set({ buttonState: 'ready' });
                }
            } else {
                set({ buttonState: 'ready' });
            }
        }
    },
}));
