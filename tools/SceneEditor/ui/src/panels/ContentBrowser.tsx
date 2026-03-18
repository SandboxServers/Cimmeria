import { useState, useEffect, useRef, useCallback, useMemo } from 'react';
import { ChevronRight, ChevronDown, Search, Package, RefreshCw } from 'lucide-react';
import { invoke } from '../lib/tauri';
import type {
  PackageScanResult,
  PackageInfo,
  ExportInfo,
  SearchResult,
  TextureThumbnail,
} from '../lib/types';

interface ContentBrowserProps {
  visible: boolean;
  cookedPcPath: string | null;
}

/** Class abbreviations for placeholder thumbnails. */
const CLASS_ABBREV: Record<string, string> = {
  StaticMesh: 'SM',
  Texture2D: 'T2D',
  Material: 'MAT',
  MaterialInstanceConstant: 'MIC',
  SkeletalMesh: 'SKM',
  SoundCue: 'SND',
  SoundNodeWave: 'WAV',
  ParticleSystem: 'PFX',
  AnimSet: 'ANM',
  AnimSequence: 'SEQ',
  PhysicsAsset: 'PHY',
  Brush: 'BSP',
  LevelStreaming: 'LVL',
};

/** Colors for class badge pills. */
const CLASS_BADGE_COLORS: Record<string, string> = {
  StaticMesh: '#6b7280',
  Texture2D: '#22c55e',
  Material: '#a855f7',
  MaterialInstanceConstant: '#c084fc',
  SkeletalMesh: '#3b82f6',
  SoundCue: '#f59e0b',
  SoundNodeWave: '#fbbf24',
  ParticleSystem: '#06b6d4',
  AnimSet: '#f97316',
  AnimSequence: '#fb923c',
  PhysicsAsset: '#ef4444',
};

/** Known class names for the filter dropdown. */
const FILTER_CLASSES = [
  'StaticMesh',
  'Texture2D',
  'Material',
  'MaterialInstanceConstant',
  'SkeletalMesh',
  'SoundCue',
  'SoundNodeWave',
  'ParticleSystem',
  'AnimSet',
  'AnimSequence',
  'PhysicsAsset',
];

/** Max concurrent thumbnail fetch requests. */
const MAX_CONCURRENT_THUMBNAILS = 4;

export function ContentBrowser({ visible, cookedPcPath }: ContentBrowserProps) {
  // ---- Package browsing state ----
  const [packages, setPackages] = useState<PackageInfo[]>([]);
  const [selectedPackage, setSelectedPackage] = useState<string | null>(null);
  const [exports, setExports] = useState<ExportInfo[]>([]);
  const [expandedPackages, setExpandedPackages] = useState<Set<string>>(new Set());

  // ---- Search state ----
  const [searchQuery, setSearchQuery] = useState('');
  const [classFilter, setClassFilter] = useState('');
  const [searchResults, setSearchResults] = useState<SearchResult[]>([]);
  const [isSearchMode, setIsSearchMode] = useState(false);

  // ---- Thumbnail cache ----
  const [thumbnailCache, setThumbnailCache] = useState<Map<string, string>>(new Map());

  // ---- Loading state ----
  const [scanning, setScanning] = useState(false);
  const [loading, setLoading] = useState(false);
  const [scanResult, setScanResult] = useState<PackageScanResult | null>(null);

  // ---- Selected asset ----
  const [selectedAsset, setSelectedAsset] = useState<string | null>(null);

  // ---- Thumbnail queue ----
  const thumbnailQueueRef = useRef<{ packageName: string; objectName: string }[]>([]);
  const activeThumbnailsRef = useRef(0);
  const thumbnailCacheRef = useRef<Map<string, string>>(new Map());

  // Keep ref in sync with state
  useEffect(() => {
    thumbnailCacheRef.current = thumbnailCache;
  }, [thumbnailCache]);

  // ---- Package tree search ----
  const [packageSearch, setPackageSearch] = useState('');

  const filteredPackages = useMemo(() => {
    if (!packageSearch.trim()) return packages;
    const q = packageSearch.toLowerCase();
    return packages.filter(p => p.name.toLowerCase().includes(q));
  }, [packages, packageSearch]);

  // ---- Group exports by class for tree view ----
  const exportsByClass = useMemo(() => {
    const map = new Map<string, ExportInfo[]>();
    for (const exp of exports) {
      if (classFilter && exp.class_name !== classFilter) continue;
      let list = map.get(exp.class_name);
      if (!list) {
        list = [];
        map.set(exp.class_name, list);
      }
      list.push(exp);
    }
    return [...map.entries()].sort((a, b) => b[1].length - a[1].length);
  }, [exports, classFilter]);

  // ---- Filtered exports for grid ----
  const gridExports = useMemo(() => {
    if (classFilter) {
      return exports.filter(e => e.class_name === classFilter);
    }
    return exports;
  }, [exports, classFilter]);

  // ---- Scan packages ----
  const handleScan = useCallback(async () => {
    if (!cookedPcPath || scanning) return;
    setScanning(true);
    try {
      const result = await invoke<PackageScanResult>('scan_packages', { cookedPcPath });
      setScanResult(result);
      const pkgList = await invoke<PackageInfo[]>('list_packages');
      setPackages(pkgList);
    } catch (e) {
      console.error('scan_packages failed:', e);
    } finally {
      setScanning(false);
    }
  }, [cookedPcPath, scanning]);

  // ---- Load package exports ----
  const handleSelectPackage = useCallback(async (packageName: string) => {
    setSelectedPackage(packageName);
    setIsSearchMode(false);
    setSelectedAsset(null);
    setLoading(true);
    try {
      const exps = await invoke<ExportInfo[]>('list_package_exports', { packageName });
      setExports(exps);
    } catch (e) {
      console.error('list_package_exports failed:', e);
      setExports([]);
    } finally {
      setLoading(false);
    }
  }, []);

  // ---- Search assets ----
  const handleSearch = useCallback(async () => {
    if (!searchQuery.trim()) {
      setIsSearchMode(false);
      setSearchResults([]);
      return;
    }
    setIsSearchMode(true);
    setLoading(true);
    try {
      const results = await invoke<SearchResult[]>('search_assets', {
        query: searchQuery,
        classFilter: classFilter || null,
        limit: 100,
      });
      setSearchResults(results);
    } catch (e) {
      console.error('search_assets failed:', e);
      setSearchResults([]);
    } finally {
      setLoading(false);
    }
  }, [searchQuery, classFilter]);

  // Trigger search on Enter key
  const handleSearchKeyDown = useCallback((e: React.KeyboardEvent) => {
    if (e.key === 'Enter') {
      e.preventDefault();
      handleSearch();
    }
  }, [handleSearch]);

  // ---- Thumbnail loading with concurrency control ----
  const processThumbnailQueue = useCallback(() => {
    while (activeThumbnailsRef.current < MAX_CONCURRENT_THUMBNAILS && thumbnailQueueRef.current.length > 0) {
      const item = thumbnailQueueRef.current.shift()!;
      const cacheKey = `${item.packageName}:${item.objectName}`;

      // Skip if already cached
      if (thumbnailCacheRef.current.has(cacheKey)) continue;

      activeThumbnailsRef.current++;
      invoke<TextureThumbnail>('get_texture_thumbnail', {
        packageName: item.packageName,
        objectName: item.objectName,
        maxSize: 96,
      })
        .then(thumb => {
          const dataUrl = `data:image/png;base64,${thumb.data_base64}`;
          setThumbnailCache(prev => {
            const next = new Map(prev);
            next.set(cacheKey, dataUrl);
            return next;
          });
        })
        .catch(() => {
          // Mark as failed so we don't retry
          setThumbnailCache(prev => {
            const next = new Map(prev);
            next.set(cacheKey, '');
            return next;
          });
        })
        .finally(() => {
          activeThumbnailsRef.current--;
          processThumbnailQueue();
        });
    }
  }, []);

  const requestThumbnail = useCallback((packageName: string, objectName: string) => {
    const cacheKey = `${packageName}:${objectName}`;
    if (thumbnailCacheRef.current.has(cacheKey)) return;
    // Avoid duplicate queue entries
    if (thumbnailQueueRef.current.some(i => i.packageName === packageName && i.objectName === objectName)) return;
    thumbnailQueueRef.current.push({ packageName, objectName });
    processThumbnailQueue();
  }, [processThumbnailQueue]);

  // ---- Toggle package tree expansion ----
  const togglePackageExpand = useCallback((name: string) => {
    setExpandedPackages(prev => {
      const next = new Set(prev);
      if (next.has(name)) next.delete(name);
      else next.add(name);
      return next;
    });
  }, []);

  // Don't render if not visible
  if (!visible) return null;

  const formatSize = (bytes: number) => {
    if (bytes < 1024) return `${bytes} B`;
    return `${(bytes / 1024).toFixed(1)} KB`;
  };

  return (
    <div className="flex h-full flex-col bg-card">
      {/* Toolbar */}
      <div className="flex items-center gap-2 border-b px-3 py-1.5">
        <Package size={12} className="shrink-0 text-muted-foreground" />
        <span className="text-xs font-medium text-foreground">Content Browser</span>

        {/* Search input */}
        <div className="relative ml-2 flex-1 max-w-[300px]">
          <Search size={12} className="absolute left-2 top-1.5 text-muted-foreground" />
          <input
            type="text"
            className="w-full rounded bg-input py-1 pl-6 pr-2 text-xs text-foreground placeholder:text-muted-foreground/60 outline-none"
            placeholder="Search assets..."
            value={searchQuery}
            onChange={e => setSearchQuery(e.target.value)}
            onKeyDown={handleSearchKeyDown}
          />
        </div>

        {/* Class filter dropdown */}
        <select
          className="rounded bg-input px-2 py-1 text-xs text-foreground outline-none"
          value={classFilter}
          onChange={e => setClassFilter(e.target.value)}
        >
          <option value="">All Classes</option>
          {FILTER_CLASSES.map(cls => (
            <option key={cls} value={cls}>{cls}</option>
          ))}
        </select>

        {/* Scan button */}
        <button
          className={`flex items-center gap-1 rounded px-2 py-1 text-xs ${
            scanning
              ? 'bg-primary/20 text-primary cursor-wait'
              : cookedPcPath
                ? 'bg-muted text-foreground hover:bg-muted/80'
                : 'bg-muted text-muted-foreground/50 cursor-not-allowed'
          }`}
          onClick={handleScan}
          disabled={scanning || !cookedPcPath}
          title={cookedPcPath ? `Scan packages in ${cookedPcPath}` : 'No CookedPC path available'}
        >
          <RefreshCw size={12} className={scanning ? 'animate-spin' : ''} />
          {scanning ? 'Scanning...' : 'Scan'}
        </button>
      </div>

      {/* Main content area */}
      <div className="flex flex-1 overflow-hidden">
        {/* Left pane: Package tree */}
        <div className="flex w-[200px] shrink-0 flex-col border-r">
          {/* Package search */}
          <div className="relative border-b px-2 py-1">
            <Search size={10} className="absolute left-3 top-2 text-muted-foreground" />
            <input
              type="text"
              className="w-full rounded bg-input py-0.5 pl-5 pr-2 text-[11px] text-foreground placeholder:text-muted-foreground/60 outline-none"
              placeholder="Filter packages..."
              value={packageSearch}
              onChange={e => setPackageSearch(e.target.value)}
            />
          </div>

          {/* Package list */}
          <div className="flex-1 overflow-y-auto subtle-scrollbar">
            {filteredPackages.length === 0 && (
              <div className="px-3 py-6 text-center text-[11px] text-muted-foreground/60">
                {packages.length === 0 ? 'Click Scan to index packages' : 'No packages match filter'}
              </div>
            )}
            {filteredPackages.map(pkg => {
              const isSelected = selectedPackage === pkg.name;
              const isExpanded = expandedPackages.has(pkg.name);
              return (
                <div key={pkg.name}>
                  <button
                    className={`flex w-full items-center gap-1 px-2 py-0.5 text-left text-[11px] ${
                      isSelected
                        ? 'bg-primary/20 text-primary'
                        : 'text-muted-foreground hover:bg-muted hover:text-foreground'
                    }`}
                    onClick={() => {
                      if (isSelected) {
                        togglePackageExpand(pkg.name);
                      } else {
                        handleSelectPackage(pkg.name);
                      }
                    }}
                  >
                    {isSelected ? (
                      isExpanded ? (
                        <ChevronDown size={10} className="shrink-0 text-muted-foreground" />
                      ) : (
                        <ChevronRight size={10} className="shrink-0 text-muted-foreground" />
                      )
                    ) : (
                      <ChevronRight size={10} className="shrink-0 text-muted-foreground/40" />
                    )}
                    <span className="truncate">{pkg.name}</span>
                    <span className="ml-auto shrink-0 font-mono text-[9px] text-muted-foreground/60">
                      {pkg.export_count}
                    </span>
                  </button>

                  {/* Expanded: show class groups */}
                  {isSelected && isExpanded && exportsByClass.map(([className, classExports]) => (
                    <div key={className} className="pl-5">
                      <div className="flex items-center gap-1 px-1 py-px text-[10px] text-muted-foreground/80">
                        <span
                          className="inline-block h-1.5 w-1.5 shrink-0 rounded-full"
                          style={{ backgroundColor: CLASS_BADGE_COLORS[className] ?? '#6b7280' }}
                        />
                        <span className="truncate">{className}</span>
                        <span className="ml-auto font-mono text-[9px] text-muted-foreground/50">
                          {classExports.length}
                        </span>
                      </div>
                    </div>
                  ))}
                </div>
              );
            })}
          </div>
        </div>

        {/* Right pane: Asset grid */}
        <div className="flex flex-1 flex-col overflow-hidden">
          {/* Grid content */}
          <div className="flex-1 overflow-y-auto p-2 subtle-scrollbar">
            {loading ? (
              <div className="flex h-full items-center justify-center">
                <div className="text-xs text-muted-foreground">Loading...</div>
              </div>
            ) : isSearchMode ? (
              /* Search results grid */
              searchResults.length === 0 ? (
                <div className="flex h-full items-center justify-center">
                  <div className="text-xs text-muted-foreground/60">
                    No results for "{searchQuery}"
                  </div>
                </div>
              ) : (
                <div className="flex flex-wrap gap-2 content-start">
                  {searchResults.map(result => {
                    const assetKey = `${result.package_name}:${result.object_name}`;
                    const isAssetSelected = selectedAsset === assetKey;
                    const thumbUrl = result.class_name === 'Texture2D'
                      ? thumbnailCache.get(assetKey)
                      : undefined;

                    // Request thumbnail for Texture2D
                    if (result.class_name === 'Texture2D' && !thumbnailCache.has(assetKey)) {
                      requestThumbnail(result.package_name, result.object_name);
                    }

                    return (
                      <AssetCard
                        key={`${result.package_name}:${result.export_index}`}
                        objectName={result.object_name}
                        className={result.class_name}
                        subtitle={result.package_name}
                        selected={isAssetSelected}
                        thumbnailUrl={thumbUrl}
                        onClick={() => setSelectedAsset(assetKey)}
                      />
                    );
                  })}
                </div>
              )
            ) : selectedPackage ? (
              /* Package exports grid */
              gridExports.length === 0 ? (
                <div className="flex h-full items-center justify-center">
                  <div className="text-xs text-muted-foreground/60">
                    {classFilter ? `No ${classFilter} assets in this package` : 'No exports in this package'}
                  </div>
                </div>
              ) : (
                <div className="flex flex-wrap gap-2 content-start">
                  {gridExports.map(exp => {
                    const assetKey = `${selectedPackage}:${exp.object_name}`;
                    const isAssetSelected = selectedAsset === assetKey;
                    const thumbUrl = exp.class_name === 'Texture2D'
                      ? thumbnailCache.get(assetKey)
                      : undefined;

                    // Request thumbnail for Texture2D
                    if (exp.class_name === 'Texture2D' && !thumbnailCache.has(assetKey) && selectedPackage) {
                      requestThumbnail(selectedPackage, exp.object_name);
                    }

                    return (
                      <AssetCard
                        key={`${selectedPackage}:${exp.index}`}
                        objectName={exp.object_name}
                        className={exp.class_name}
                        subtitle={formatSize(exp.serial_size)}
                        selected={isAssetSelected}
                        thumbnailUrl={thumbUrl}
                        onClick={() => setSelectedAsset(assetKey)}
                      />
                    );
                  })}
                </div>
              )
            ) : (
              <div className="flex h-full items-center justify-center">
                <div className="text-center text-xs text-muted-foreground/60">
                  {packages.length > 0
                    ? 'Select a package to browse assets'
                    : 'Click Scan to index packages'}
                </div>
              </div>
            )}
          </div>
        </div>
      </div>

      {/* Status bar */}
      <div className="flex items-center gap-3 border-t px-3 py-1 text-[10px] text-muted-foreground">
        {scanResult && (
          <span>{scanResult.package_count} packages, {scanResult.total_exports.toLocaleString()} exports</span>
        )}
        {selectedPackage && !isSearchMode && (
          <span>{gridExports.length} assets in {selectedPackage}</span>
        )}
        {isSearchMode && (
          <span>{searchResults.length} search results</span>
        )}
        {loading && <span>Loading...</span>}
        {scanning && <span>Scanning packages...</span>}
      </div>
    </div>
  );
}

// ---- Asset card sub-component ----

function AssetCard({
  objectName,
  className,
  subtitle,
  selected,
  thumbnailUrl,
  onClick,
}: {
  objectName: string;
  className: string;
  subtitle: string;
  selected: boolean;
  thumbnailUrl: string | undefined;
  onClick: () => void;
}) {
  const abbrev = CLASS_ABBREV[className] ?? className.slice(0, 3).toUpperCase();
  const badgeColor = CLASS_BADGE_COLORS[className] ?? '#6b7280';
  const hasThumbnail = thumbnailUrl && thumbnailUrl.length > 0;

  return (
    <button
      className={`flex w-24 flex-col items-center rounded-md border p-1.5 text-center transition-colors ${
        selected
          ? 'border-amber-400 bg-amber-400/10'
          : 'border-transparent bg-muted/50 hover:bg-muted hover:border-border'
      }`}
      onClick={onClick}
      title={`${objectName}\n${className}\n${subtitle}`}
    >
      {/* Thumbnail area */}
      <div className="flex h-16 w-20 items-center justify-center overflow-hidden rounded bg-black/20">
        {hasThumbnail ? (
          <img
            src={thumbnailUrl}
            alt={objectName}
            className="max-h-full max-w-full object-contain"
            draggable={false}
          />
        ) : (
          <span className="text-sm font-bold text-muted-foreground/30">{abbrev}</span>
        )}
      </div>

      {/* Object name */}
      <div className="mt-1 w-full truncate text-[10px] text-foreground" title={objectName}>
        {objectName}
      </div>

      {/* Class badge + size */}
      <div className="flex w-full items-center justify-between gap-1">
        <span
          className="truncate rounded-sm px-1 text-[8px] font-medium text-white"
          style={{ backgroundColor: badgeColor }}
        >
          {abbrev}
        </span>
        <span className="truncate text-[8px] text-muted-foreground/60">{subtitle}</span>
      </div>
    </button>
  );
}
