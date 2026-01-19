import { create } from 'zustand';
import { check } from '@tauri-apps/plugin-updater';
import { ask } from '@tauri-apps/plugin-dialog';
import { relaunch } from '@tauri-apps/plugin-process';
import i18next from 'i18next';

interface LauncherUpdateState {
    checking: boolean;
    updateAvailable: boolean;
    checkSelfUpdate: () => Promise<void>;
}

export const useLauncherStore = create<LauncherUpdateState>((set) => ({
    checking: false,
    updateAvailable: false,

    checkSelfUpdate: async () => {
        // Prevent concurrent checks (React StrictMode or multiple triggers)
        if (useLauncherStore.getState().checking) return;

        set({ checking: true });
        try {
            const update = await check();
            if (update) {
                set({ updateAvailable: true });
                const yes = await ask(
                    i18next.t('self_update.message', { version: update.version }),
                    {
                        title: i18next.t('self_update.title'),
                        kind: 'info'
                    }
                );

                if (yes) {
                    await update.downloadAndInstall();
                    await relaunch();
                }
            }
        } catch (error) {
            console.error('Failed to check for launcher updates:', error);
        } finally {
            set({ checking: false });
        }
    }
}));
