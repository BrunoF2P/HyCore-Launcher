import { UserProfile } from "./UserProfile";
import { LaunchSection } from "../../../features/game/components/LaunchSection";

export const ActionBar = () => {
    return (
        <footer className="h-24 w-full flex items-center justify-between px-14 z-10 shrink-0 bg-black/20 backdrop-blur-sm">
            <UserProfile />
            <LaunchSection />
        </footer>
    );
};
