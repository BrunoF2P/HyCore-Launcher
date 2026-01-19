import { useState, useEffect } from "react";
import { Plus, Package, Calendar, ArrowRight, Trash2, CheckCircle2, FilePlus } from "lucide-react";
import { useModStore } from "../store/useModStore";

export const ProfileManager = () => {
    const {
        profiles,
        activeProfile,
        fetchProfiles,
        fetchActiveProfile,
        createProfile,
        deleteProfile,
        setActiveProfile
    } = useModStore();

    const [newProfileName, setNewProfileName] = useState("");
    const [isCreating, setIsCreating] = useState(false);
    const [switchingPack, setSwitchingPack] = useState<string | null>(null);

    useEffect(() => {
        fetchProfiles();
        fetchActiveProfile();
    }, []);

    const handleCreate = async (empty: boolean) => {
        if (!newProfileName.trim()) return;
        setIsCreating(true);
        try {
            await createProfile(newProfileName, empty);
            setNewProfileName("");
        } catch (error) {
            alert("Erro ao criar perfil: " + error);
        } finally {
            setIsCreating(false);
        }
    };

    const handleDelete = async (name: string) => {
        if (name === "Default") return;
        if (!confirm(`Deseja excluir o perfil "${name}"?`)) return;
        try {
            await deleteProfile(name);
        } catch (error) {
            console.error("Failed to delete profile", error);
        }
    };

    const handleActivate = async (name: string) => {
        if (name === activeProfile) return;
        setSwitchingPack(name);
        try {
            await setActiveProfile(name);
        } catch (error) {
            alert("Erro ao ativar perfil: " + error);
        } finally {
            setSwitchingPack(null);
        }
    };

    return (
        <div className="p-8 space-y-8 animate-in fade-in slide-in-from-bottom-4 duration-500 max-w-7xl mx-auto">
            <div className="flex flex-col md:flex-row md:items-center justify-between gap-6">
                <div>
                    <h2 className="text-3xl font-black tracking-tight uppercase px-1 bg-gradient-to-r from-sky-400 to-blue-400 bg-clip-text text-transparent">Perfis de Mods</h2>
                    <p className="text-white/40 text-sm mt-1">Crie instâncias separadas para diferentes estilos de jogo.</p>
                </div>

                <div className="flex flex-col gap-2">
                    <div className="flex gap-2 bg-white/5 p-1.5 rounded-2xl border border-white/5 backdrop-blur-xl">
                        <input
                            type="text"
                            placeholder="Nome do perfil..."
                            value={newProfileName}
                            onChange={(e) => setNewProfileName(e.target.value)}
                            className="bg-black/20 border border-white/10 rounded-xl px-4 py-2.5 text-sm focus:outline-none focus:border-sky-500/50 transition-all w-48 md:w-64"
                        />
                        <button
                            onClick={() => handleCreate(false)}
                            disabled={isCreating || !newProfileName.trim()}
                            className="flex items-center gap-2 bg-sky-500 text-black px-4 py-2.5 rounded-xl font-bold text-[10px] uppercase tracking-wider hover:bg-sky-400 disabled:opacity-30 transition-all whitespace-nowrap cursor-pointer"
                            title="Cria uma cópia do seu estado atual"
                        >
                            <Plus size={16} />
                            Snapshot
                        </button>
                        <button
                            onClick={() => handleCreate(true)}
                            disabled={isCreating || !newProfileName.trim()}
                            className="flex items-center gap-2 bg-white/10 text-white px-4 py-2.5 rounded-xl font-bold text-[10px] uppercase tracking-wider hover:bg-white/20 disabled:opacity-30 transition-all whitespace-nowrap cursor-pointer"
                            title="Cria um perfil limpo sem mods"
                        >
                            <FilePlus size={16} />
                            Vazio
                        </button>
                    </div>
                </div>
            </div>

            <div className="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-3 gap-6">
                {profiles.map((pack) => (
                    <div
                        key={pack.name}
                        className={`group relative bg-[#141822]/80 backdrop-blur-sm border rounded-3xl p-6 transition-all hover:-translate-y-1 hover:shadow-2xl ${activeProfile === pack.name
                            ? "border-sky-500/50 shadow-sky-500/10 ring-1 ring-sky-500/20"
                            : "border-white/10 hover:border-sky-500/40"
                            }`}
                    >
                        {activeProfile === pack.name && (
                            <div className="absolute -top-3 left-6 bg-sky-500 text-black text-[9px] font-black uppercase tracking-[0.2em] px-3 py-1 rounded-full shadow-lg shadow-sky-500/20">
                                Ativo Agora
                            </div>
                        )}

                        <div className="flex justify-between items-start mb-6">
                            <div className={`p-4 rounded-2xl border transition-colors ${activeProfile === pack.name
                                ? "bg-sky-500 text-black border-sky-500/20"
                                : "bg-sky-500/10 text-sky-500 border-sky-500/10"
                                }`}>
                                <Package className="w-6 h-6" />
                            </div>
                            {pack.name !== "Default" && (
                                <button
                                    onClick={() => handleDelete(pack.name)}
                                    className="p-2.5 text-white/10 hover:text-red-400 hover:bg-red-400/10 rounded-xl transition-all cursor-pointer"
                                >
                                    <Trash2 size={16} />
                                </button>
                            )}
                        </div>

                        <h3 className="text-xl font-bold mb-1 group-hover:text-sky-400 transition-colors">{pack.name}</h3>
                        <div className="flex items-center gap-3 text-white/40 text-[10px] uppercase font-bold tracking-widest mb-8">
                            <span className="bg-white/5 px-2 py-0.5 rounded-md text-sky-400/80">
                                {pack.modCount} Mods
                            </span>
                            <div className="h-1 w-1 rounded-full bg-white/10" />
                            <span className="flex items-center gap-1.5 opacity-60">
                                <Calendar size={12} />
                                {new Date(pack.createdAt).toLocaleDateString()}
                            </span>
                        </div>

                        <button
                            onClick={() => handleActivate(pack.name)}
                            disabled={!!switchingPack || activeProfile === pack.name}
                            className={`flex items-center justify-center gap-3 w-full py-4 rounded-2xl font-black uppercase tracking-widest text-[10px] transition-all relative overflow-hidden cursor-pointer ${activeProfile === pack.name
                                ? "bg-sky-500/10 text-sky-400 border border-sky-500/20 cursor-default"
                                : switchingPack === pack.name
                                    ? "bg-sky-500/20 text-sky-400 border border-sky-500/30"
                                    : "bg-white/5 text-white/70 hover:bg-sky-500 hover:text-black hover:shadow-lg hover:shadow-sky-500/20 active:scale-[0.98]"
                                }`}
                        >
                            {switchingPack === pack.name ? (
                                <>
                                    <div className="w-4 h-4 border-2 border-sky-400 border-t-transparent rounded-full animate-spin" />
                                    Ativando...
                                </>
                            ) : activeProfile === pack.name ? (
                                <>
                                    <CheckCircle2 size={14} />
                                    Perfil Ativo
                                </>
                            ) : (
                                <>
                                    Ativar Perfil
                                    <ArrowRight className="w-4 h-4 transition-transform group-hover:translate-x-1" />
                                </>
                            )}
                        </button>
                    </div>
                ))}
            </div>
        </div>
    );
};
