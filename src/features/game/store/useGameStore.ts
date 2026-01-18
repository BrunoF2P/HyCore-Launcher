import { create } from 'zustand';
import { invoke } from '@tauri-apps/api/core';

type ButtonState = 'idle' | 'checking' | 'ready' | 'update_available' | 'updating' | 'launching';

interface GameState {
    buttonState: ButtonState;
    updateAvailable: boolean;
    latestVersion: number;
    setButtonState: (state: ButtonState) => void;
    checkForUpdates: () => Promise<void>;
    launchGame: () => Promise<void>;
}

export const useGameStore = create<GameState>((set) => ({
    buttonState: 'idle',
    updateAvailable: false,
    latestVersion: 0,

    setButtonState: (state) => set({ buttonState: state }),

    checkForUpdates: async () => {
        set({ buttonState: 'checking' });

        try {
            const [updateAvailable, latestVersion] = await invoke<[boolean, number]>('check_for_game_update');

            set({
                updateAvailable,
                latestVersion,
                buttonState: updateAvailable ? 'update_available' : 'ready',
            });
        } catch (err) {
            console.error('Failed to check for updates:', err);
            set({ buttonState: 'ready' });
        }
    },

    launchGame: async () => {
        set({ buttonState: 'launching' });

        try {
            await invoke('launch_game');
            setTimeout(() => {
                set({ buttonState: 'ready' });
            }, 2000);
        } catch (err) {
            console.error('Failed to launch game:', err);
            alert(`Failed to launch game: ${err}`);
            set({ buttonState: 'ready' });
        }
    },
}));
