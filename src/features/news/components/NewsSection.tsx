import { useEffect } from "react";
import { useTranslation } from "react-i18next";
import { useNewsStore } from "../store/useNewsStore";
import { FeaturedNews } from "./FeaturedNews";
import { NewsList } from "./NewsList";

export const NewsSection = () => {
    const { t } = useTranslation();
    const { news, loading, fetchNews } = useNewsStore();

    useEffect(() => {
        fetchNews();
    }, [fetchNews]);

    const featuredNews = news[0];
    const otherNews = news.slice(1);

    if (loading) {
        return (
            <div className="flex items-center justify-center p-10 bg-black/45 backdrop-blur-md border border-white/10 rounded-lg text-zinc-400">
                {t('news.loading')}
            </div>
        );
    }

    if (!featuredNews) {
        return (
            <div className="flex items-center justify-center p-10 bg-black/45 backdrop-blur-md border border-white/10 rounded-lg text-zinc-400">
                {t('news.no_internet')}
            </div>
        );
    }

    return (
        <section className="mb-2 w-[28rem] flex flex-col gap-4">
            <FeaturedNews item={featuredNews} />
            <div>
                <NewsList items={otherNews} />
            </div>
        </section>
    );
};
