import { openUrl } from "@tauri-apps/plugin-opener";
import { NewsItem } from "../types";

interface NewsListProps {
    items: NewsItem[];
}

export const NewsList = ({ items }: NewsListProps) => {
    return (
        <div className="bg-black/45 backdrop-blur-md border border-white/10 rounded-lg p-2.5">
            {items.map((item, idx) => (
                <button
                    key={idx}
                    className="w-full flex justify-between items-center px-5 py-2.5 cursor-pointer transition-colors border-b border-white/5 last:border-none hover:bg-white/5 group text-left focus-visible:outline-none focus-visible:bg-white/10 focus-visible:ring-inset focus-visible:ring-1 focus-visible:ring-hyamber/30"
                    onClick={() => openUrl(item.link)}
                    aria-label={item.title}
                >
                    <span className="text-sm font-medium tracking-tight truncate max-w-[17.5rem] group-hover:text-hyamber transition-colors">
                        {item.title}
                    </span>
                    <span className="text-[10px] text-zinc-500 uppercase tracking-wider font-bold">
                        {item.date}
                    </span>
                </button>
            ))}
        </div>
    );
};
