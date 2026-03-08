import { useEffect, useMemo, useState } from 'react';
import {
  type CardRelevance,
  buildMissionCardLibrarySections,
  type PrimitiveFamily,
  type ScenarioTemplate,
} from './missionCardCatalog';

const favoritesStorageKey = 'cimmeria.chain-editor.card-favorites';
const recentsStorageKey = 'cimmeria.chain-editor.card-recents';
const maxRecentCards = 8;

const familyLabel: Record<PrimitiveFamily, string> = {
  anchor: 'Sequence',
  trigger: 'Trigger',
  condition: 'Condition',
  action: 'Action',
  counter: 'Counter',
};

const familyTint: Record<PrimitiveFamily, { chip: string; text: string }> = {
  anchor: {
    chip: 'rgba(255,94,91,0.16)',
    text: '#ffd0cf',
  },
  trigger: {
    chip: 'rgba(19,162,164,0.16)',
    text: '#8ae5e2',
  },
  condition: {
    chip: 'rgba(245,170,49,0.16)',
    text: '#ffd38a',
  },
  action: {
    chip: 'rgba(34,197,94,0.16)',
    text: '#c7ffd5',
  },
  counter: {
    chip: 'rgba(148,163,184,0.16)',
    text: '#d4dce5',
  },
};

function readStoredArray(key: string) {
  if (typeof window === 'undefined') {
    return [] as string[];
  }

  try {
    const rawValue = window.localStorage.getItem(key);
    if (!rawValue) {
      return [] as string[];
    }

    const parsed = JSON.parse(rawValue);
    return Array.isArray(parsed) ? parsed.filter((value): value is string => typeof value === 'string') : [];
  } catch {
    return [] as string[];
  }
}

function CatalogCard({
  favorite,
  onAdd,
  onToggleFavorite,
  template,
}: {
  favorite: boolean;
  onAdd: (template: ScenarioTemplate) => void;
  onToggleFavorite: (templateId: string) => void;
  template: ScenarioTemplate;
}) {
  return (
    <div className="rounded-[24px] border border-white/8 bg-[rgba(255,255,255,0.03)] p-4 transition-colors hover:border-[rgba(245,170,49,0.3)] hover:bg-[rgba(245,170,49,0.08)]">
      <div className="flex items-start justify-between gap-3">
        <button
          className="flex-1 text-left"
          onClick={() => onAdd(template)}
          type="button"
        >
          <span className="text-sm font-medium text-white">{template.title}</span>
        </button>
        <div className="flex items-center gap-2">
          <button
            className={`rounded-full border px-3 py-1 text-[10px] font-semibold uppercase tracking-[0.18em] ${
              favorite
                ? 'border-[rgba(245,170,49,0.38)] bg-[rgba(245,170,49,0.16)] text-[#ffd38a]'
                : 'border-white/8 bg-white/4 text-[rgba(224,231,239,0.72)]'
            }`}
            onClick={() => onToggleFavorite(template.id)}
            type="button"
          >
            {favorite ? 'Pinned' : 'Pin'}
          </button>
          <span
            className="rounded-full px-3 py-1 text-[11px] font-semibold uppercase tracking-[0.18em]"
            style={{
              backgroundColor: `${template.accent}22`,
              color: template.accent,
            }}
          >
            {template.stage}
          </span>
        </div>
      </div>
      <button className="mt-2 w-full text-left" onClick={() => onAdd(template)} type="button">
        <p className="text-sm leading-6 text-[rgba(224,231,239,0.76)]">{template.detail}</p>
        <div className="mt-3 flex items-center justify-between gap-3">
          <div>
            <p className="text-xs uppercase tracking-[0.2em] text-[rgba(160,174,192,0.72)]">
              {template.scenario}
            </p>
            <p className="mt-2 text-xs uppercase tracking-[0.18em] text-[rgba(94,184,179,0.8)]">
              {template.relevance.join(' + ')}
            </p>
          </div>
          <span
            className="rounded-full px-3 py-1 text-[10px] font-semibold uppercase tracking-[0.2em]"
            style={{
              backgroundColor: familyTint[template.family].chip,
              color: familyTint[template.family].text,
            }}
          >
            {familyLabel[template.family]}
          </span>
        </div>
      </button>
    </div>
  );
}

export default function MissionCardLibrary({
  onAdd,
  selectedChainName,
}: {
  onAdd: (template: ScenarioTemplate) => void;
  selectedChainName: string;
}) {
  const [search, setSearch] = useState('');
  const [family, setFamily] = useState<PrimitiveFamily | 'all'>('all');
  const [relevance, setRelevance] = useState<CardRelevance | 'all'>('all');
  const [favoritesOnly, setFavoritesOnly] = useState(false);
  const [recentsOnly, setRecentsOnly] = useState(false);
  const [favoriteIds, setFavoriteIds] = useState<string[]>(() => readStoredArray(favoritesStorageKey));
  const [recentIds, setRecentIds] = useState<string[]>(() => readStoredArray(recentsStorageKey));

  useEffect(() => {
    window.localStorage.setItem(favoritesStorageKey, JSON.stringify(favoriteIds));
  }, [favoriteIds]);

  useEffect(() => {
    window.localStorage.setItem(recentsStorageKey, JSON.stringify(recentIds));
  }, [recentIds]);

  const sections = useMemo(
    () =>
      buildMissionCardLibrarySections({
        family,
        favoriteIds,
        recentIds,
        relevance,
        search,
        showFavoritesOnly: favoritesOnly,
        showRecentsOnly: recentsOnly,
      }),
    [favoriteIds, family, favoritesOnly, recentIds, recentsOnly, relevance, search],
  );

  const handleToggleFavorite = (templateId: string) => {
    setFavoriteIds((current) =>
      current.includes(templateId)
        ? current.filter((entry) => entry !== templateId)
        : [templateId, ...current],
    );
  };

  const handleAdd = (template: ScenarioTemplate) => {
    onAdd(template);
    setRecentIds((current) => [template.id, ...current.filter((entry) => entry !== template.id)].slice(0, maxRecentCards));
  };

  return (
    <section className="rounded-[32px] border border-white/8 bg-[rgba(9,18,28,0.8)] p-6 shadow-[0_24px_80px_rgba(0,0,0,0.22)]">
      <div className="flex flex-wrap items-start gap-4">
        <div className="min-w-[320px] flex-1">
          <p className="text-xs font-semibold uppercase tracking-[0.24em] text-[rgba(160,174,192,0.72)]">
            Mission Card Library
          </p>
          <p className="mt-2 max-w-4xl text-sm leading-6 text-[rgba(224,231,239,0.76)]">
            Search the catalog, pin frequent cards, and reuse recent inserts instead of re-scanning every section each time.
          </p>
        </div>
        <div className="flex min-h-[88px] min-w-[360px] flex-1 items-center justify-between rounded-[24px] border border-[rgba(245,170,49,0.2)] bg-[linear-gradient(135deg,rgba(245,170,49,0.14),rgba(19,162,164,0.08))] px-5 py-4 text-sm text-[rgba(224,231,239,0.9)] shadow-[inset_0_1px_0_rgba(255,255,255,0.06)]">
          <div>
            <p className="text-xs font-semibold uppercase tracking-[0.22em] text-[rgba(245,170,49,0.82)]">
              Adding Cards Into
            </p>
            <p className="mt-2 text-lg font-semibold tracking-[-0.03em] text-white">
              {selectedChainName}
            </p>
          </div>
          <span className="rounded-full border border-white/10 bg-[rgba(9,18,28,0.46)] px-4 py-2 text-xs font-semibold uppercase tracking-[0.2em] text-[rgba(224,231,239,0.76)]">
            Active target
          </span>
        </div>
      </div>

      <div className="mt-6 grid gap-3 xl:grid-cols-[minmax(0,1.4fr)_repeat(4,minmax(0,0.6fr))]">
        <label className="rounded-[24px] border border-white/8 bg-[rgba(255,255,255,0.03)] px-4 py-3">
          <span className="text-[10px] font-semibold uppercase tracking-[0.2em] text-[rgba(160,174,192,0.72)]">
            Search
          </span>
          <input
            className="mt-2 w-full bg-transparent text-sm text-white outline-none placeholder:text-[rgba(160,174,192,0.56)]"
            onChange={(event) => setSearch(event.target.value)}
            placeholder="Find by title, detail, tag, or property"
            type="search"
            value={search}
          />
        </label>

        <label className="rounded-[24px] border border-white/8 bg-[rgba(255,255,255,0.03)] px-4 py-3">
          <span className="text-[10px] font-semibold uppercase tracking-[0.2em] text-[rgba(160,174,192,0.72)]">
            Family
          </span>
          <select
            className="mt-2 w-full bg-transparent text-sm text-white outline-none"
            onChange={(event) => setFamily(event.target.value as PrimitiveFamily | 'all')}
            value={family}
          >
            <option value="all">All families</option>
            <option value="anchor">Sequence</option>
            <option value="trigger">Trigger</option>
            <option value="condition">Condition</option>
            <option value="action">Action</option>
            <option value="counter">Counter</option>
          </select>
        </label>

        <label className="rounded-[24px] border border-white/8 bg-[rgba(255,255,255,0.03)] px-4 py-3">
          <span className="text-[10px] font-semibold uppercase tracking-[0.2em] text-[rgba(160,174,192,0.72)]">
            Relevance
          </span>
          <select
            className="mt-2 w-full bg-transparent text-sm text-white outline-none"
            onChange={(event) => setRelevance(event.target.value as CardRelevance | 'all')}
            value={relevance}
          >
            <option value="all">All scopes</option>
            <option value="mission">Mission</option>
            <option value="space">Space</option>
          </select>
        </label>

        <button
          className={`rounded-[24px] border px-4 py-3 text-left text-sm transition-colors ${
            favoritesOnly
              ? 'border-[rgba(245,170,49,0.3)] bg-[rgba(245,170,49,0.12)] text-[#ffd38a]'
              : 'border-white/8 bg-[rgba(255,255,255,0.03)] text-[rgba(224,231,239,0.76)]'
          }`}
          onClick={() => {
            setFavoritesOnly((current) => !current);
            setRecentsOnly(false);
          }}
          type="button"
        >
          Favorites only
        </button>

        <button
          className={`rounded-[24px] border px-4 py-3 text-left text-sm transition-colors ${
            recentsOnly
              ? 'border-[rgba(94,184,179,0.3)] bg-[rgba(94,184,179,0.12)] text-[#a7f0eb]'
              : 'border-white/8 bg-[rgba(255,255,255,0.03)] text-[rgba(224,231,239,0.76)]'
          }`}
          onClick={() => {
            setRecentsOnly((current) => !current);
            setFavoritesOnly(false);
          }}
          type="button"
        >
          Recent only
        </button>
      </div>

      <div className="mt-6 grid gap-4 2xl:grid-cols-2">
        {sections.length ? (
          sections.map((section) => (
            <section
              className="rounded-[28px] border border-white/8 bg-[rgba(255,255,255,0.03)] p-5"
              key={section.id}
            >
              <p className="text-xs font-semibold uppercase tracking-[0.22em] text-[rgba(160,174,192,0.72)]">
                {section.title}
              </p>
              <div className="mt-4 grid gap-3 md:grid-cols-2">
                {section.templates.map((template) => (
                  <CatalogCard
                    favorite={favoriteIds.includes(template.id)}
                    key={template.id}
                    onAdd={handleAdd}
                    onToggleFavorite={handleToggleFavorite}
                    template={template}
                  />
                ))}
              </div>
            </section>
          ))
        ) : (
          <section className="rounded-[28px] border border-dashed border-white/10 bg-[rgba(255,255,255,0.02)] p-10 text-center text-sm text-[rgba(224,231,239,0.72)] 2xl:col-span-2">
            No cards match the current search/filter combination.
          </section>
        )}
      </div>
    </section>
  );
}
