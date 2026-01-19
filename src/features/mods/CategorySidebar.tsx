import { useState, useEffect } from "react";
import { invoke } from "@tauri-apps/api/core";
import { ModCategory } from "./types";
import { ChevronRight, LayoutGrid } from "lucide-react";

interface CategorySidebarProps {
    onSelectCategory: (categoryId: number | undefined) => void;
    activeCategoryId?: number;
}

export const CategorySidebar = ({ onSelectCategory, activeCategoryId }: CategorySidebarProps) => {
    const [categories, setCategories] = useState<ModCategory[]>([]);
    const [isLoading, setIsLoading] = useState(true);

    useEffect(() => {
        invoke<ModCategory[]>("get_categories")
            .then((data) => {
                // Filter only relevant categories for Hytale if needed, or show all
                setCategories(data.sort((a, b) => a.name.localeCompare(b.name)));
            })
            .catch(console.error)
            .finally(() => setIsLoading(false));
    }, []);

    if (isLoading) {
        return (
            <div className="w-64 flex flex-col gap-2 p-4 animate-pulse">
                {[...Array(8)].map((_, i) => (
                    <div key={i} className="h-10 bg-white/5 rounded-lg" />
                ))}
            </div>
        );
    }

    return (
        <aside className="w-64 flex flex-col gap-1 p-4 overflow-y-auto custom-scrollbar bg-black/20 border-r border-white/5 h-full">
            <h3 className="text-[10px] uppercase font-bold tracking-widest text-white/30 px-3 mb-2 flex items-center justify-between">
                Categorias
            </h3>

            <button
                onClick={() => onSelectCategory(undefined)}
                className={`flex items-center justify-between px-3 py-2.5 rounded-lg transition-all group cursor-pointer ${!activeCategoryId
                    ? "bg-gradient-to-r from-sky-500/20 to-blue-500/20 text-sky-400 border border-sky-500/20 shadow-lg shadow-sky-500/5 font-bold"
                    : "text-white/40 hover:text-white/80 hover:bg-white/5"
                    }`}
            >
                <div className="flex items-center gap-3">
                    <LayoutGrid size={16} className={!activeCategoryId ? "text-sky-400" : "opacity-50"} />
                    <span className="text-xs">Todos os Mods</span>
                </div>
                {!activeCategoryId && <ChevronRight className="w-3 h-3" />}
            </button>

            <div className="my-2 h-px bg-white/5 mx-3" />

            {categories.map((cat) => (
                <button
                    key={cat.id}
                    onClick={() => onSelectCategory(cat.id)}
                    className={`flex items-center justify-between px-3 py-2.5 rounded-lg transition-all group cursor-pointer ${activeCategoryId === cat.id
                        ? "bg-sky-500/20 text-sky-400 border border-sky-500/20 shadow-lg shadow-sky-500/5 font-bold"
                        : "text-white/40 hover:text-white/80 hover:bg-white/5"
                        }`}
                >
                    <div className="flex items-center gap-3 overflow-hidden">
                        {cat.iconUrl ? (
                            <img
                                src={cat.iconUrl}
                                alt=""
                                className={`w-4 h-4 transition-all ${activeCategoryId === cat.id ? "opacity-100 scale-110" : "opacity-30 group-hover:opacity-70"}`}
                            />
                        ) : (
                            <div className={`w-1.5 h-1.5 rounded-full ${activeCategoryId === cat.id ? "bg-sky-400 shadow-[0_0_8px_rgba(56,189,248,0.5)]" : "bg-white/10"}`} />
                        )}
                        <span className="text-xs truncate">{cat.name}</span>
                    </div>
                    {activeCategoryId === cat.id && <ChevronRight className="w-3 h-3" />}
                </button>
            ))}
        </aside>
    );
};
