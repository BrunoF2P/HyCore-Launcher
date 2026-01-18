import { ExternalLink } from "lucide-react";
import { openUrl } from "@tauri-apps/plugin-opener";
import { NewsItem } from "../types";

interface FeaturedNewsProps {
    item: NewsItem;
}

export const FeaturedNews = ({ item }: FeaturedNewsProps) => {
    return (
        <div
            className="bg-black/45 backdrop-blur-md border border-white/10 rounded-lg overflow-hidden cursor-pointer transition-all duration-300 hover:scale-[1.02] hover:border-hyamber group shrink-0"
            onClick={() => openUrl(item.link)}
        >
            <div className="w-full h-40 overflow-hidden rounded-t-lg">
                <img
                    src={item.image_url}
                    alt={item.title}
                    className="w-full h-full object-cover rounded-t-lg"
                />
            </div>
            <div className="p-5">
                <h3 className="text-lg font-bold text-white mb-1 tracking-tight group-hover:text-hyamber transition-colors">
                    {item.title}
                </h3>
                <div className="flex justify-between items-center text-xs text-zinc-400">
                    <span>{item.date}</span>
                    <ExternalLink className="w-4 h-4" />
                </div>
            </div>
        </div>
    );
};
