import { CalendarDays, CheckSquare2, Plus, LockKeyhole } from 'lucide-react';
import { Button } from '@/components/ui/button';
import { MiniCalendar } from '@/components/calendar/mini-calendar';
import { useCalendarStore } from '@/stores/calendar-store';
import type { Category } from '@/types/calendar';

interface SidebarProps {
  categories: Category[];
  firstDay: 0 | 1;
  ready: boolean;
  showCompleted: boolean;
  saving: boolean;
  toggleCompleted: () => void;
}

export function Sidebar({ categories, firstDay, ready, showCompleted, saving, toggleCompleted }: SidebarProps) {
  const { visibleKinds, toggleKind, hiddenCategoryIds, toggleCategory } = useCalendarStore();
  return (
    <aside id="sidebar" className="sidebar" aria-label="Barra lateral">
      <div>
        <Button disabled={!ready} className="create-button" onClick={() => useCalendarStore.getState().openCreate()}><Plus /> Criar</Button>
      </div>
      <MiniCalendar firstDay={firstDay} />
      <section className="sidebar-section" aria-labelledby="show-heading">
        <h2 id="show-heading">Mostrar no calendário</h2>
        <label className="filter-row"><input type="checkbox" checked={visibleKinds.includes('task')} onChange={() => toggleKind('task')} /><CheckSquare2 aria-hidden="true" /> Tarefas</label>
        <label className="filter-row"><input type="checkbox" checked={visibleKinds.includes('event')} onChange={() => toggleKind('event')} /><CalendarDays aria-hidden="true" /> Eventos</label>
        <label className="filter-row completed-filter"><input type="checkbox" checked={showCompleted} disabled={!ready || saving} onChange={toggleCompleted} /> Incluir concluídas</label>
      </section>
      <section className="sidebar-section" aria-labelledby="categories-heading">
        <div className="form-section-heading"><h2 id="categories-heading">Minhas categorias</h2><Button variant="ghost" size="sm" aria-label="Gerenciar categorias" disabled={!ready} onClick={() => useCalendarStore.getState().setCategoriesOpen(true)}><Plus /></Button></div>
        {!ready && <p className="text-xs text-muted-foreground">Categorias aguardando o SQLite.</p>}
        {categories.map((category) => <label className="filter-row" key={category.id}>
          <input type="checkbox" style={{ accentColor: category.color }} checked={!hiddenCategoryIds.includes(category.id)} onChange={() => toggleCategory(category.id)} />
          <span className="category-dot" style={{ backgroundColor: category.color }} />{category.name}
        </label>)}
      </section>
      <div className="offline-note"><LockKeyhole aria-hidden="true" /><div><strong>Seu tempo, seu espaço.</strong><p>Dados locais. Sem conta e sem nuvem.</p></div></div>
    </aside>
  );
}
