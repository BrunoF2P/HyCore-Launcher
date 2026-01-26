import { useEffect } from "react";
import { useTranslation } from "react-i18next";
import { useNewsStore } from "../store/useNewsStore";
import { FeaturedNews } from "./FeaturedNews";
import { NewsList } from "./NewsList";

export const NewsSection = () => {
    const { t } = useTranslation();
    const news = useNewsStore(state => state.news);
    const loading = useNewsStore(state => state.loading);
    const fetchNews = useNewsStore(state => state.fetchNews);

    useEffect(() => {
        fetchNews();
    }, [fetchNews]);

    const featuredNews = news[0];
    const otherNews = news.slice(1);

    if (loading) {
        return (
            <section className="mb-2 w-full max-w-md flex flex-col gap-4 animate-pulse">
                {/* Featured News Skeleton */}
                <div className="bg-white/5 border border-white/5 rounded-lg h-[16.5rem]"></div>
                {/* News List Skeleton */}
                <div className="bg-white/5 border border-white/5 rounded-lg p-2.5 flex flex-col gap-2">
                    {[1, 2, 3].map((i) => (
                        <div key={i} className="h-10 bg-white/5 rounded w-full"></div>
                    ))}
                </div>
            </section>
        );
    }

    if (!featuredNews) {
        return (
            <div className="flex flex-col items-center justify-center p-10 bg-black/45 backdrop-blur-md border border-white/10 rounded-lg text-zinc-400 w-full max-w-md text-center gap-2">
                <div className="w-12 h-12 rounded-full bg-white/5 flex items-center justify-center mb-2">
                    <svg className="w-6 h-6 opacity-20" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                        <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M12 9v2m0 4h.01m-6.938 4h13.856c1.54 0 2.502-1.667 1.732-3L13.732 4c-.77-1.333-2.694-1.333-3.464 0L3.34 16c-.77 1.333.192 3 1.732 3z" />
                    </svg>
                </div>
                <p className="font-medium text-sm text-white/60">{t('news.no_internet')}</p>
                <button
                    onClick={() => fetchNews()}
                    className="text-xs text-hyamber hover:text-hyamber-light font-bold uppercase tracking-wider mt-2 transition-colors"
                >
                    {t('common.retry') || "Retry"}
                </button>
            </div>
        );
    }

    return (
        <section className="mb-2 w-full max-w-md flex flex-col gap-4">
            <FeaturedNews item={featuredNews} />
            <div>
                <NewsList items={otherNews} />
            </div>
        </section>
    );
};
