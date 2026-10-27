import { Button } from '@/components/ui/button';

export function CalendarColors({ text, background, onChange }: {
  text: string | null; background: string | null;
  onChange: (field: 'calendar_text_color' | 'calendar_background_color', value: string | null) => void;
}) {
  return <>
    <h4 className="mb-3 text-sm font-medium">Cores do calendário</h4>
    <p className="field-help">Personalize o texto e o fundo das visões de mês, semana e ano. As cores das categorias são mantidas.</p>
    <div className="form-grid">
      <label className="form-field">Cor do texto
        <input type="color" value={text ?? '#29313e'} onChange={(event) => onChange('calendar_text_color', event.target.value)} />
        <span className="field-help">{text ?? 'Padrão do tema'}</span>
      </label>
      <label className="form-field">Cor do fundo
        <input type="color" value={background ?? '#ffffff'} onChange={(event) => onChange('calendar_background_color', event.target.value)} />
        <span className="field-help">{background ?? 'Padrão do tema'}</span>
      </label>
    </div>
    <div className="calendar-color-preview" style={{ color: text ?? 'var(--foreground)', background: background ?? 'var(--surface)' }} aria-label="Prévia das cores do calendário">
      <span>Segunda-feira</span><strong>27</strong><span>Prévia das cores</span>
    </div>
    <Button variant="outline" size="sm" onClick={() => { onChange('calendar_text_color', null); onChange('calendar_background_color', null); }}>Restaurar cores do tema</Button>
  </>;
}
