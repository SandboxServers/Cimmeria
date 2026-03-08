import { describe, expect, it } from 'vitest';
import {
  allMissionCardTemplates,
  buildMissionCardLibrarySections,
  matchesMissionCardSearch,
} from './missionCardCatalog';

describe('missionCardCatalog', () => {
  it('matches card search against title, detail, tags, and property content', () => {
    const template = allMissionCardTemplates.find((entry) => entry.id === 'trigger-enter-region');
    expect(template).toBeDefined();
    expect(matchesMissionCardSearch(template!, 'region8')).toBe(true);
    expect(matchesMissionCardSearch(template!, 'space event')).toBe(true);
    expect(matchesMissionCardSearch(template!, 'Zone.RegionN')).toBe(true);
    expect(matchesMissionCardSearch(template!, 'not-a-real-card')).toBe(false);
  });

  it('builds sections with favorites and recents floated to the top', () => {
    const sections = buildMissionCardLibrarySections({
      family: 'all',
      favoriteIds: ['action-accept-mission'],
      recentIds: ['trigger-interact-tag', 'action-accept-mission'],
      relevance: 'all',
      search: '',
      showFavoritesOnly: false,
      showRecentsOnly: false,
    });

    expect(sections[0]?.id).toBe('favorites');
    expect(sections[0]?.templates.map((template) => template.id)).toEqual([
      'action-accept-mission',
    ]);
    expect(sections[1]?.id).toBe('recents');
    expect(sections[1]?.templates.map((template) => template.id)).toEqual([
      'trigger-interact-tag',
      'action-accept-mission',
    ]);
  });

  it('applies family and relevance filters together', () => {
    const sections = buildMissionCardLibrarySections({
      family: 'trigger',
      favoriteIds: [],
      recentIds: [],
      relevance: 'space',
      search: '',
      showFavoritesOnly: false,
      showRecentsOnly: false,
    });

    expect(sections.every((section) => section.templates.every((template) => template.family === 'trigger'))).toBe(true);
    expect(
      sections.flatMap((section) => section.templates.map((template) => template.id)),
    ).toEqual(['trigger-enter-region', 'trigger-interact-tag']);
  });

  it('supports favorites-only and recents-only views', () => {
    const favoritesOnly = buildMissionCardLibrarySections({
      family: 'all',
      favoriteIds: ['condition-archetype'],
      recentIds: ['condition-archetype', 'action-advance-step'],
      relevance: 'all',
      search: '',
      showFavoritesOnly: true,
      showRecentsOnly: false,
    });

    expect(favoritesOnly).toHaveLength(1);
    expect(favoritesOnly[0]?.id).toBe('favorites');
    expect(favoritesOnly[0]?.templates.map((template) => template.id)).toEqual([
      'condition-archetype',
    ]);

    const recentsOnly = buildMissionCardLibrarySections({
      family: 'all',
      favoriteIds: ['condition-archetype'],
      recentIds: ['condition-archetype', 'action-advance-step'],
      relevance: 'all',
      search: '',
      showFavoritesOnly: false,
      showRecentsOnly: true,
    });

    expect(recentsOnly[0]?.id).toBe('recents');
    expect(recentsOnly[0]?.templates.map((template) => template.id)).toEqual([
      'condition-archetype',
      'action-advance-step',
    ]);
    expect(recentsOnly).toHaveLength(1);
  });
});
