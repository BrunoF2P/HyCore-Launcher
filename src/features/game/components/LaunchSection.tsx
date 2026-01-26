import { useEffect } from "react";
import { useTranslation } from "react-i18next";
import { useGameStore } from "../store/useGameStore";
import { useUpdateStore } from "../store/updateStore";
import { CheckCircle2, Loader2 } from "lucide-react";
import { VersionSelector } from "./VersionSelector";

export const LaunchSection = () => {
    const { t } = useTranslation();
    const buttonState = useGameStore(state => state.buttonState);
    const launchGame = useGameStore(state => state.launchGame);
    const checkForUpdates = useGameStore(state => state.checkForUpdates);
    const setButtonState = useGameStore(state => state.setButtonState);

    const status = useUpdateStore(state => state.status);
    const isUpdating = useUpdateStore(state => state.isUpdating);
    const startUpdate = useUpdateStore(state => state.startUpdate);

    useEffect(() => {
        if (!isUpdating && status.stage === 'done') {
            setButtonState('idle');
            checkForUpdates();
        }
    }, [isUpdating, status.stage]);

    const handleButtonClick = async () => {
        if (buttonState === 'update_available') {
            await startUpdate();
        } else if (buttonState === 'ready') {
            await launchGame();
        }
    };

    const getButtonContent = () => {
        if (isUpdating && status.stage === 'download') {
            return (
                <div className="relative w-full h-full flex items-center justify-center">
                    <div
                        className="absolute inset-0 bg-white/30 transition-all duration-300 pointer-events-none"
                        style={{ width: `${status.progress}%` }}
                    />
                    <span className="relative z-10">
                        {Math.round(status.progress)}% - {t('update.downloading')}
                    </span>
                </div>
            );
        }

        if (isUpdating) {
            return (
                <span>
                    {status.message || t('update.preparing')}
                </span>
            );
        }

        switch (buttonState) {
            case 'checking':
                return (
                    <div className="flex items-center gap-2">
                        <Loader2 className="w-5 h-5 animate-spin" />
                        <span>{t('footer.checking')}</span>
                    </div>
                );
            case 'update_available':
                return t('footer.update');
            case 'launching':
                return (
                    <div className="flex items-center gap-2">
                        <Loader2 className="w-5 h-5 animate-spin" />
                        <span>{t('footer.launching')}</span>
                    </div>
                );
            case 'ready':
            default:
                return t('footer.launch');
        }
    };

    const isDisabled =
        buttonState === 'checking' ||
        buttonState === 'launching' ||
        (isUpdating && status.stage !== 'error' && status.stage !== 'done');

    const buttonColor = buttonState === 'update_available'
        ? 'from-amber-400 via-amber-500 to-amber-600'
        : 'from-hyamber-light via-hyamber to-hyamber-dark';

    return (
        <div className="flex items-center gap-4">
            {isUpdating && status.stage === 'download' && (
                <div className="flex flex-col items-end text-[10px] font-mono whitespace-nowrap text-zinc-500">
                    <span className="text-zinc-400 font-bold">{status.message}</span>
                </div>
            )}

            {!isUpdating && status.stage === 'done' && (
                <div className="flex items-center gap-2 text-sky-400 animate-in fade-in slide-in-from-right-2 duration-500">
                    <CheckCircle2 className="w-5 h-5 drop-shadow-[0_0_8px_rgba(56,189,248,0.3)]" />
                    <span className="text-[10px] font-bold uppercase tracking-wider">{t('update.complete')}</span>
                </div>
            )}

            <VersionSelector />

            <button
                className={`group relative overflow-hidden flex items-center justify-center w-64 h-16 rounded bg-gradient-to-br ${buttonColor} text-black text-2xl font-black uppercase tracking-tighter transition-all hover:scale-[1.02] hover:shadow-[0_0_25px_rgba(255,179,0,0.3)] active:scale-95 disabled:opacity-50 disabled:cursor-not-allowed ${isDisabled ? 'cursor-wait' : 'cursor-pointer'}`}
                onClick={handleButtonClick}
                disabled={isDisabled}
            >
                {getButtonContent()}
            </button>
        </div>
    );
};
