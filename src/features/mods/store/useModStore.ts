import { create } from 'zustand';
import { invoke } from '@tauri-apps/api/core';
import { InstalledMod, Modpack, CurseForgeMod, ModCategory } from '../types';

interface ModState {
    installedMods: InstalledMod[];
    activeProfile: string;
    profiles: Modpack[];
    categories: ModCategory[];
    installingIds: number[];
    loading: boolean;
    error: string | null;

    // Actions
    fetchInstalledMods: () => Promise<void>;
    fetchActiveProfile: () => Promise<void>;
    fetchProfiles: () => Promise<void>;
    fetchCategories: () => Promise<void>;
    checkUpdates: () => Promise<void>;
    installMod: (mod: CurseForgeMod) => Promise<void>;
    removeMod: (modId: string) => Promise<void>;
    toggleMod: (modId: string, enabled: boolean) => Promise<void>;
    updateMod: (mod: InstalledMod) => Promise<void>;
    createProfile: (name: string, empty: boolean) => Promise<void>;
    deleteProfile: (name: string) => Promise<void>;
    setActiveProfile: (name: string) => Promise<void>;
}

export const useModStore = create<ModState>((set, get) => ({
    installedMods: [],
    activeProfile: 'Default',
    profiles: [],
    categories: [],
    installingIds: [],
    loading: false,
    error: null,

    fetchInstalledMods: async () => {
        try {
            const mods = await invoke<InstalledMod[]>('get_installed_mods');
            set({ installedMods: mods });
        } catch (err: any) {
            console.error('Failed to fetch installed mods:', err);
        }
    },

    fetchActiveProfile: async () => {
        try {
            const active = await invoke<string>('get_active_profile');
            set({ activeProfile: active });
        } catch (err: any) {
            console.error('Failed to fetch active profile:', err);
        }
    },

    fetchProfiles: async () => {
        try {
            const list = await invoke<Modpack[]>('list_profiles');
            set({ profiles: list });
        } catch (err: any) {
            console.error('Failed to fetch profiles:', err);
        }
    },

    fetchCategories: async () => {
        try {
            const list = await invoke<ModCategory[]>('get_categories');
            set({ categories: list.sort((a, b) => a.name.localeCompare(b.name)) });
        } catch (err: any) {
            console.error('Failed to fetch categories:', err);
        }
    },

    checkUpdates: async () => {
        try {
            const updatedIds = await invoke<string[]>('check_mods_updates');
            if (updatedIds.length > 0) {
                await get().fetchInstalledMods();
            }
        } catch (err: any) {
            console.error('Failed to check for mod updates:', err);
        }
    },

    installMod: async (mod: CurseForgeMod) => {
        set(state => ({ installingIds: [...state.installingIds, mod.id] }));
        try {
            await invoke('install_mod_cf', { modId: mod.id, fileId: null });
            await get().fetchInstalledMods();
        } catch (err: any) {
            console.error('Install failed:', err);
            throw err;
        } finally {
            set(state => ({ installingIds: state.installingIds.filter(id => id !== mod.id) }));
        }
    },

    removeMod: async (modId: string) => {
        try {
            await invoke('remove_mod', { modId });
            await get().fetchInstalledMods();
        } catch (err: any) {
            console.error('Remove failed:', err);
            throw err;
        }
    },

    toggleMod: async (modId: string, enabled: boolean) => {
        try {
            await invoke('toggle_mod', { modId, enabled });
            await get().fetchInstalledMods();
        } catch (err: any) {
            console.error('Toggle failed:', err);
            throw err;
        }
    },

    updateMod: async (mod: InstalledMod) => {
        if (!mod.curseForgeId || !mod.latestFileId) return;
        set(state => ({ installingIds: [...state.installingIds, mod.curseForgeId!] }));
        try {
            await invoke('install_mod_cf', {
                modId: mod.curseForgeId,
                fileId: mod.latestFileId
            });
            await get().fetchInstalledMods();
        } catch (err: any) {
            console.error('Update failed:', err);
            throw err;
        } finally {
            set(state => ({ installingIds: state.installingIds.filter(id => id !== mod.curseForgeId) }));
        }
    },

    createProfile: async (name: string, empty: boolean) => {
        try {
            await invoke('create_profile', { name, empty });
            await get().fetchProfiles();
        } catch (err: any) {
            console.error('Failed to create profile:', err);
            throw err;
        }
    },

    deleteProfile: async (name: string) => {
        try {
            await invoke('delete_profile', { name });
            await get().fetchProfiles();
            if (get().activeProfile === name) {
                await get().fetchActiveProfile();
                await get().fetchInstalledMods();
            }
        } catch (err: any) {
            console.error('Failed to delete profile:', err);
            throw err;
        }
    },

    setActiveProfile: async (name: string) => {
        try {
            await invoke('set_active_profile', { name });
            set({ activeProfile: name });
            await get().fetchInstalledMods();
            await get().fetchProfiles();
        } catch (err: any) {
            console.error('Failed to set active profile:', err);
            throw err;
        }
    }
}));
