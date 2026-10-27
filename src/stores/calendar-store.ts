import { create } from 'zustand';
import type { CalendarEntry, CalendarView, EditorRequest, EntryKind } from '@/types/calendar';
import { navigatePeriod } from '../features/calendar/dates.ts';

interface CalendarState {
  selectedDate: Date;
  editorRequest: EditorRequest | null;
  revision: number;
  categoriesOpen: boolean;
  openCreate: (request?: EditorRequest) => void;
  openEditor: (entry: CalendarEntry) => void;
  closeEditor: () => void;
  touch: () => void;
  setCategoriesOpen: (open: boolean) => void;
  view: CalendarView;
  sidebarOpen: boolean;
  visibleKinds: EntryKind[];
  hiddenCategoryIds: string[];
  selectDate: (date: Date) => void;
  setView: (view: CalendarView) => void;
  openWeek: (date: Date) => void;
  navigate: (direction: -1 | 1) => void;
  today: () => void;
  toggleSidebar: () => void;
  toggleKind: (kind: EntryKind) => void;
  toggleCategory: (id: string) => void;
}

export const useCalendarStore = create<CalendarState>((set) => ({
  selectedDate: new Date(),
  editorRequest: null, revision: 0, categoriesOpen: false,
  openCreate: (request) => set((state) => ({ editorRequest: request ?? { start: state.selectedDate, allDay: true } })),
  openEditor: (entry) => set({ editorRequest: { entry } }),
  closeEditor: () => set({ editorRequest: null }),
  touch: () => set((state) => ({ revision: state.revision + 1 })),
  setCategoriesOpen: (categoriesOpen) => set({ categoriesOpen }),
  view: 'month',
  sidebarOpen: true,
  visibleKinds: ['task', 'event'],
  hiddenCategoryIds: [],
  selectDate: (date) => set({ selectedDate: date }),
  setView: (view) => set({ view }),
  openWeek: (date) => set({ selectedDate: date, view: 'week' }),
  navigate: (direction) => set((state) => ({ selectedDate: navigatePeriod(state.selectedDate, state.view, direction) })),
  today: () => set({ selectedDate: new Date() }),
  toggleSidebar: () => set((state) => ({ sidebarOpen: !state.sidebarOpen })),
  toggleKind: (kind) => set((state) => ({
    visibleKinds: state.visibleKinds.includes(kind)
      ? state.visibleKinds.filter((value) => value !== kind) : [...state.visibleKinds, kind],
  })),
  toggleCategory: (id) => set((state) => ({
    hiddenCategoryIds: state.hiddenCategoryIds.includes(id)
      ? state.hiddenCategoryIds.filter((value) => value !== id) : [...state.hiddenCategoryIds, id],
  })),
}));
