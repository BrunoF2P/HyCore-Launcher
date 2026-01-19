import { useTranslation } from "react-i18next";

export const TopNav = () => {
    const { t } = useTranslation();
    return (
        <nav className="relative flex justify-between items-start p-10 z-10">
            <div>
                <img src="/logo.png" alt={t('common.logo_alt')} className="h-20 drop-shadow-xl" />
            </div>
        </nav>
    );
};
