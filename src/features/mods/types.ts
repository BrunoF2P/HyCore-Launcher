export interface ModLogo {
    id: number;
    modId: number;
    title: string;
    description: string;
    thumbnailUrl: string;
    url: string;
}

export interface ModCategory {
    id: number;
    name: string;
    slug: string;
    url: string;
    iconUrl: string;
}

export interface ModAuthor {
    id: number;
    name: string;
    url: string;
}

export interface ModScreenshot {
    id: number;
    modId: number;
    title: string;
    description: string;
    thumbnailUrl: string;
    url: string;
}

export interface ModFile {
    id: number;
    modId: number;
    displayName: string;
    fileName: string;
    fileLength: number;
    downloadUrl?: string; // Optional in Rust struct
    fileDate: string;
    releaseType: number;
}

export interface CurseForgeMod {
    id: number;
    gameId: number;
    name: string;
    slug: string;
    summary: string;
    downloadCount: number;
    dateCreated: string;
    dateModified: string;
    dateReleased: string;
    logo?: ModLogo;
    screenshots: ModScreenshot[];
    categories: ModCategory[];
    authors: ModAuthor[];
    latestFiles: ModFile[];
    mainFileId: number;
    allowModDistribution: boolean;
}

export interface SearchResult {
    mods: CurseForgeMod[];
    totalCount: number;
    pageIndex: number;
    pageSize: number;
}

export interface InstalledMod {
    id: string; // "cf-12345"
    name: string;
    slug?: string;
    version: string;
    author: string;
    description: string;
    downloadUrl?: string;
    curseForgeId?: number;
    fileId?: number;
    enabled: boolean;
    installedAt: string;
    updatedAt: string;
    filePath: string;
    iconUrl?: string;
    downloads?: number;
    category?: string;
    latestVersion?: string;
    latestFileId?: number;
}

export interface SearchModsParams {
    query?: string;
    categoryId?: number;
    sortField?: number;
    sortOrder?: string;
    pageSize?: number;
    index?: number;
}
