import { create } from 'zustand';
import { check } from '@tauri-apps/plugin-updater';
import { ask } from '@tauri-apps/plugin-dialog';
import { relaunch } from '@tauri-apps/plugin-process';

interface LauncherUpdateState {
    checking: boolean;
    updateAvailable: boolean;
    checkSelfUpdate: () => Promise<void>;
}

export const useLauncherStore = create<LauncherUpdateState>((set) => ({
    checking: false,
    updateAvailable: false,

    checkSelfUpdate: async () => {
        set({ checking: true });
        try {
            const update = await check();
            if (update?.available) {
                set({ updateAvailable: true });
                const yes = await ask(
                    `New launcher version ${update.version} is available! Do you want to update now?`,
                    { title: 'Launcher Update', kind: 'info' }
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
