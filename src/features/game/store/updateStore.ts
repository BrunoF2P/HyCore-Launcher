import { create } from 'zustand';
import { invoke } from '@tauri-apps/api/core';
import { listen } from '@tauri-apps/api/event';

interface UpdateStatus {
    stage: 'idle' | 'checking' | 'butler' | 'download' | 'install' | 'error' | 'done';
    progress: number;
    message: string;
}

interface UpdateState {
    status: UpdateStatus;
    isUpdating: boolean;
    error: string | null;

    checkRequirements: () => Promise<boolean>;
    startUpdate: () => Promise<void>;
    reset: () => void;
    cleanup: () => void;
}

export const useUpdateStore = create<UpdateState>((set) => {
    const unsubscribe = listen<UpdateStatus>('update-status', (event) => {
        set((state) => ({
            status: {
                ...state.status,
                ...event.payload,
            },
        }));
    });

    return {
        status: {
            stage: 'idle',
            progress: 0,
            message: '',
        },
        isUpdating: false,
        error: null,

        checkRequirements: async () => {
            try {
                const reqs = await invoke<any>('check_update_requirements');
                return reqs.meets_requirements;
            } catch (err) {
                set({ error: String(err) });
                return false;
            }
        },

        startUpdate: async () => {
            set({ isUpdating: true, error: null });
            try {
                await invoke('start_game_update');
                set({ isUpdating: false });
            } catch (err) {
                invoke('log_updater_error', { error: String(err) });
                set({
                    isUpdating: false,
                    error: String(err),
                    status: {
                        stage: 'error',
                        progress: 0,
                        message: 'Update failed. Technical details in updater.log'
                    }
                });
            }
        },

        reset: () => set({
            status: { stage: 'idle', progress: 0, message: '' },
            isUpdating: false,
            error: null,
        }),

        cleanup: () => {
            unsubscribe.then(fn => fn());
        }
    };
});
