import { openUrl } from "@tauri-apps/plugin-opener";
import { NewsItem } from "../types";

interface NewsListProps {
    items: NewsItem[];
}

export const NewsList = ({ items }: NewsListProps) => {
    return (
        <div className="bg-black/45 backdrop-blur-md border border-white/10 rounded-lg p-2.5">
            {items.map((item, idx) => (
                <div
                    key={idx}
                    className="flex justify-between items-center px-5 py-2.5 cursor-pointer transition-colors border-b border-white/5 last:border-none hover:bg-white/5 group"
                    onClick={() => openUrl(item.link)}
                >
                    <span className="text-sm font-medium tracking-tight truncate max-w-[17.5rem] group-hover:text-hyamber transition-colors">
                        {item.title}
                    </span>
                    <span className="text-[10px] text-zinc-500 uppercase tracking-wider font-bold">
                        {item.date}
                    </span>
                </div>
            ))}
        </div>
    );
};
