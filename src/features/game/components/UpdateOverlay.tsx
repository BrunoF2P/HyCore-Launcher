import React from 'react';
import { useUpdateStore } from '../store/updateStore';
import { Package, Loader2, CheckCircle2, AlertCircle } from "lucide-react";
import { useTranslation } from 'react-i18next';

export const UpdateOverlay: React.FC = () => {
    const { t } = useTranslation();
    const { status, isUpdating } = useUpdateStore();

    const isShowing = isUpdating || status.stage === 'error';
    if (!isShowing || status.stage === 'idle' || status.stage === 'download' || status.stage === 'butler') return null;

    const getIcon = () => {
        const size = status.stage === 'error' ? 'w-8 h-8' : 'w-10 h-10';
        switch (status.stage) {
            case 'checking': return <Loader2 className={`${size} text-blue-400 animate-spin`} />;
            case 'install': return <Package className={`${size} text-blue-500 animate-pulse`} />;
            case 'done': return <CheckCircle2 className={`${size} text-emerald-500`} />;
            case 'error': return <AlertCircle className={`${size} text-red-500`} />;
            default: return <Loader2 className={`${size} text-blue-400 animate-spin`} />;
        }
    };

    return (
        <div className="fixed inset-0 z-50 flex items-center justify-center bg-black/80 backdrop-blur-md animate-in fade-in duration-500">
            <div className={`
                ${status.stage === 'error' ? 'max-w-xs p-6' : 'max-w-sm p-10'} 
                w-full bg-hyzinc-900/60 backdrop-blur-xl rounded-2xl border border-white/5 shadow-2xl flex flex-col items-center text-center gap-6
            `}>
                <div className={`${status.stage === 'error' ? 'p-3' : 'p-4'} bg-white/5 rounded-full`}>
                    {getIcon()}
                </div>

                <div className="space-y-1">
                    <h2 className={`${status.stage === 'error' ? 'text-lg' : 'text-xl'} font-bold text-white tracking-tight`}>
                        {status.stage === 'done' ? t('update.complete') :
                            status.stage === 'error' ? t('update.error') :
                                t('update.title')}
                    </h2>
                    {status.stage !== 'error' && (
                        <p className="text-zinc-400 text-xs">
                            {t(`update.stages.${status.stage}`) || t('update.preparing')}
                        </p>
                    )}
                </div>

                {status.stage !== 'done' && status.stage !== 'error' && (
                    <div className="w-full h-1 bg-hyzinc-800 rounded-full overflow-hidden">
                        <div
                            className="h-full bg-gradient-to-r from-blue-500 via-indigo-500 to-purple-500 transition-all duration-300 ease-out"
                            style={{ width: `${status.progress}%` }}
                        />
                    </div>
                )}

                {status.stage === 'done' && (
                    <button
                        onClick={() => window.location.reload()}
                        className="px-6 py-2 bg-emerald-500 hover:bg-emerald-600 text-white rounded-lg font-bold text-sm transition-all shadow-lg shadow-emerald-500/25 active:scale-95"
                    >
                        {t('common.continue')}
                    </button>
                )}

                {status.stage === 'error' && (
                    <div className="flex flex-col gap-4 w-full">
                        <div className="text-zinc-300 text-sm leading-relaxed bg-red-500/10 p-4 rounded-xl border border-red-500/20 mb-2">
                            {status.message || t('update.error_desc')}
                        </div>
                        <button
                            onClick={() => useUpdateStore.getState().reset()}
                            className="w-full py-3 bg-red-500 hover:bg-red-600 text-white rounded-xl font-bold text-sm transition-all shadow-lg shadow-red-500/20 active:scale-95"
                        >
                            {t('common.close')}
                        </button>
                    </div>
                )}
            </div>
        </div>
    );
};
