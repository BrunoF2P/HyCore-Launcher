import { create } from 'zustand';
import { invoke } from '@tauri-apps/api/core';
import { NewsItem } from '../types';

interface NewsState {
    news: NewsItem[];
    loading: boolean;
    error: string | null;
    fetchNews: () => Promise<void>;
}

export const useNewsStore = create<NewsState>((set) => ({
    news: [],
    loading: true,
    error: null,

    fetchNews: async () => {
        set({ loading: true, error: null });
        try {
            const data = await invoke<NewsItem[]>("get_news");
            set({ news: data, loading: false });
        } catch (error) {
            console.error("Failed to fetch news:", error);
            set({
                error: error instanceof Error ? error.message : String(error),
                loading: false
            });
        }
    },
}));
