import { useState } from 'react';
import { useForm } from 'react-hook-form';
import { zodResolver } from '@hookform/resolvers/zod';
import { invoke } from '@tauri-apps/api/core';
import { Pencil, Trash2 } from 'lucide-react';
import { Dialog, DialogContent, DialogDescription, DialogTitle } from '@/components/ui/dialog';
import { Button } from '@/components/ui/button';
import { useCalendarStore } from '@/stores/calendar-store';
import type { Category } from '@/types/calendar';
import { categorySchema, type CategoryForm } from '@/features/categories/schema';

export function CategoryManager({ categories, onChanged }: { categories: Category[]; onChanged: () => void }) {
  const open = useCalendarStore((state) => state.categoriesOpen);
  const setOpen = useCalendarStore((state) => state.setCategoriesOpen);
  const [id,setId] = useState<string | null>(null);
  const form=useForm<CategoryForm>({resolver:zodResolver(categorySchema),defaultValues:{name:'',color:'#1A73E8'}});
  const [busy,setBusy] = useState(false); const [error,setError] = useState<string | null>(null);
  const [deleting,setDeleting] = useState<Category | null>(null);
  async function mutate(action: () => Promise<unknown>) {
    setBusy(true);setError(null);try {await action();onChanged();useCalendarStore.getState().touch();setId(null);form.reset({name:'',color:'#1A73E8'});setDeleting(null);}catch(error){setError(String(error));}finally{setBusy(false);}
  }
  return <Dialog open={open} onOpenChange={setOpen}><DialogContent>
    <DialogTitle className="text-xl font-medium">Categorias</DialogTitle><DialogDescription className="mt-2 text-sm text-muted-foreground">Ao excluir uma categoria, os registros são preservados sem categoria.</DialogDescription>
    {error && <p className="form-error" role="alert">{error}</p>}
    <div className="category-list">{categories.map((category) => <div className="category-manager-row" key={category.id}>
      <span className="category-dot" style={{backgroundColor:category.color}} />{category.name}
      <Button variant="ghost" size="icon" aria-label={`Editar ${category.name}`} disabled={busy} onClick={() => {setId(category.id);form.reset({name:category.name,color:category.color});}}><Pencil /></Button>
      <Button variant="ghost" size="icon" aria-label={`Excluir ${category.name}`} disabled={busy} onClick={() => setDeleting(category)}><Trash2 /></Button>
    </div>)}</div>
    {deleting && <div className="delete-confirmation"><p className="text-sm">Excluir a categoria “{deleting.name}”? Os registros ficarão sem categoria.</p><div className="entry-actions"><Button variant="outline" size="sm" disabled={busy} onClick={() => setDeleting(null)}>Cancelar</Button><Button className="danger-button" size="sm" disabled={busy} onClick={() => {void mutate(() => invoke('delete_category',{id:deleting.id}));}}>Confirmar exclusão</Button></div></div>}
    <form className="entry-form" noValidate onSubmit={(event) => {void form.handleSubmit((values)=>mutate(() => invoke('save_category',{id,...values})))(event);}}><fieldset disabled={busy}>
      <label className="form-field">Nome<input maxLength={80} {...form.register('name')} />{form.formState.errors.name&&<span role="alert" className="field-error">{form.formState.errors.name.message}</span>}</label>
      <label className="form-field">Cor<input type="color" {...form.register('color')} />{form.formState.errors.color&&<span role="alert" className="field-error">{form.formState.errors.color.message}</span>}</label>
      <div className="form-actions">{id && <Button variant="outline" onClick={() => {setId(null);form.reset({name:'',color:'#1A73E8'});}}>Cancelar edição</Button>}<Button type="submit" disabled={busy}>{id ? 'Salvar categoria' : 'Criar categoria'}</Button></div>
    </fieldset>
    </form>
  </DialogContent></Dialog>;
}
