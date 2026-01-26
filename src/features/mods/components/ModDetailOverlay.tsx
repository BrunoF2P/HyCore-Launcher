import { useState } from "react";
import { X, Download, User, Calendar, ExternalLink, Hash } from "lucide-react";
import { CurseForgeMod } from "../types";
import { useModStore } from "../store/useModStore";

interface ModDetailOverlayProps {
    mod: CurseForgeMod;
    onClose: () => void;
}

export const ModDetailOverlay = ({ mod, onClose }: ModDetailOverlayProps) => {
    const installMod = useModStore(state => state.installMod);
    const installingIds = useModStore(state => state.installingIds);
    const installedMods = useModStore(state => state.installedMods);
    const isInstalling = installingIds.includes(mod.id);
    const isInstalled = installedMods.some(m => m.curseForgeId === mod.id);

    const onInstall = (m: CurseForgeMod) => {
        installMod(m).catch(err => alert("Erro ao instalar: " + err));
    };
    const [activeScreenshot, setActiveScreenshot] = useState(mod.screenshots[0]?.url || mod.logo?.url);

    return (
        <div className="fixed inset-0 z-50 flex items-center justify-center p-10 animate-in fade-in zoom-in-95 duration-300">
            {/* Backdrop */}
            <div className="absolute inset-0 bg-black/80 backdrop-blur-md" onClick={onClose} />

            {/* Content Container */}
            <div className="relative w-full max-w-6xl h-full max-h-[850px] bg-[#141822] border border-white/10 rounded-2xl shadow-2xl flex flex-col overflow-hidden">
                {/* Header Section */}
                <div className="relative h-[450px] shrink-0 overflow-hidden">
                    <div className="absolute inset-0 opacity-80 scale-105">
                        <img src={activeScreenshot} alt="" className="w-full h-full object-cover" />
                    </div>
                    <div className="absolute inset-0 bg-gradient-to-t from-[#141822] via-[#141822]/20 to-transparent" />

                    <button
                        onClick={onClose}
                        className="absolute top-6 right-6 p-2 rounded-full bg-black/40 text-white/70 hover:text-white hover:bg-black/60 transition-all z-10 cursor-pointer"
                    >
                        <X className="w-6 h-6" />
                    </button>

                    <div className="absolute bottom-10 left-10 right-10 flex items-end justify-between">
                        <div className="flex items-end gap-6">
                            {mod.logo && (
                                <img
                                    src={mod.logo.url}
                                    alt={mod.name}
                                    className="w-32 h-32 rounded-2xl border-4 border-white/5 shadow-2xl object-cover bg-black/20"
                                />
                            )}
                            <div className="mb-2">
                                <h1 className="text-4xl font-black tracking-tight mb-2 text-white">
                                    {mod.name}
                                </h1>
                                <div className="flex items-center gap-4 text-white/50 text-sm">
                                    <div className="flex items-center gap-1.5">
                                        <User className="w-4 h-4" />
                                        <span>{mod.authors[0]?.name}</span>
                                    </div>
                                    <div className="h-4 w-px bg-white/10" />
                                    <div className="flex items-center gap-1.5">
                                        <Download className="w-4 h-4" />
                                        <span>{mod.downloadCount.toLocaleString()} downloads</span>
                                    </div>
                                    <div className="h-4 w-px bg-white/10" />
                                    <div className="flex items-center gap-1.5">
                                        <Hash className="w-4 h-4" />
                                        <span>v{mod.latestFiles[0]?.displayName || "1.0.0"}</span>
                                    </div>
                                </div>
                            </div>
                        </div>

                        <button
                            onClick={() => onInstall(mod)}
                            disabled={isInstalling || isInstalled}
                            className={`flex items-center gap-2 px-8 py-4 rounded-xl font-bold uppercase tracking-widest transition-all cursor-pointer ${isInstalled
                                ? "bg-sky-500/10 text-sky-400 border border-sky-500/20 cursor-default"
                                : "bg-gradient-to-r from-sky-500 to-blue-500 text-white hover:scale-105 active:scale-95 shadow-lg shadow-sky-500/20"
                                }`}
                        >
                            {isInstalling ? (
                                <div className="w-5 h-5 border-2 border-white/30 border-t-white rounded-full animate-spin" />
                            ) : isInstalled ? (
                                "Já Instalado"
                            ) : (
                                <>
                                    <Download className="w-5 h-5" />
                                    Instalar Agora
                                </>
                            )}
                        </button>
                    </div>
                </div>

                {/* Main Content Area */}
                <div className="flex-1 overflow-hidden flex">
                    <div className="flex-1 overflow-y-auto p-10 custom-scrollbar space-y-10">
                        {/* Screenshots */}
                        {mod.screenshots.length > 0 && (
                            <div className="space-y-4">
                                <h3 className="text-lg font-bold flex items-center gap-2">
                                    Galerie de Imagens
                                </h3>
                                <div className="grid grid-cols-4 gap-4">
                                    {mod.screenshots.map((ss) => (
                                        <button
                                            key={ss.id}
                                            onClick={() => setActiveScreenshot(ss.url)}
                                            className={`relative aspect-video rounded-lg overflow-hidden border-2 transition-all cursor-pointer ${activeScreenshot === ss.url ? "border-sky-500 scale-105" : "border-transparent opacity-60 hover:opacity-100"
                                                }`}
                                        >
                                            <img src={ss.thumbnailUrl} alt="" className="w-full h-full object-cover" />
                                        </button>
                                    ))}
                                </div>
                            </div>
                        )}

                        {/* Description */}
                        <div className="space-y-4">
                            <h3 className="text-lg font-bold">Descrição</h3>
                            <div className="prose prose-invert max-w-none text-white/70 leading-relaxed bg-white/5 p-6 rounded-2xl">
                                {mod.summary}
                                <p className="mt-4 text-sm italic">
                                    Dica: Veja a descrição completa no CurseForge clicando no link lateral.
                                </p>
                            </div>
                        </div>
                    </div>

                    {/* Meta Sidebar */}
                    <div className="w-80 bg-black/20 p-10 border-l border-white/5 space-y-8 overflow-y-auto custom-scrollbar">
                        <section>
                            <h4 className="text-[10px] uppercase font-bold tracking-widest text-white/30 mb-4">Informações Técnicas</h4>
                            <div className="space-y-4">
                                <div className="flex flex-col gap-1">
                                    <span className="text-xs text-white/40">Última Atualização</span>
                                    <div className="flex items-center gap-2 text-sm text-white/80">
                                        <Calendar className="w-4 h-4 opacity-50" />
                                        {new Date(mod.dateModified).toLocaleDateString()}
                                    </div>
                                </div>
                                <div className="flex flex-col gap-1">
                                    <span className="text-xs text-white/40">Criado em</span>
                                    <div className="flex items-center gap-2 text-sm text-white/80">
                                        <Calendar className="w-4 h-4 opacity-50" />
                                        {new Date(mod.dateCreated).toLocaleDateString()}
                                    </div>
                                </div>
                            </div>
                        </section>

                        <section>
                            <h4 className="text-[10px] uppercase font-bold tracking-widest text-white/30 mb-4">Categorias</h4>
                            <div className="flex flex-wrap gap-2">
                                {mod.categories.map((cat) => (
                                    <span key={cat.id} className="px-3 py-1.5 rounded-lg bg-white/5 text-[10px] font-bold text-white/70 flex items-center gap-2">
                                        {cat.iconUrl && <img src={cat.iconUrl} alt="" className="w-3 h-3 opacity-50" />}
                                        {cat.name}
                                    </span>
                                ))}
                            </div>
                        </section>

                        <section className="pt-4">
                            <a
                                href={`https://www.curseforge.com/hytale/mods/${mod.slug}`}
                                target="_blank"
                                rel="noreferrer"
                                className="flex items-center justify-center gap-2 w-full py-4 rounded-xl border border-white/10 text-white/50 hover:text-white hover:bg-white/5 transition-all text-xs font-bold uppercase tracking-widest"
                            >
                                <ExternalLink className="w-4 h-4" />
                                Ver no CurseForge
                            </a>
                        </section>
                    </div>
                </div>
            </div>
        </div>
    );
};
