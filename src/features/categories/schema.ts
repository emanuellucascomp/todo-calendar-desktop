import { z } from 'zod';
export const categorySchema = z.object({
  name:z.string().trim().min(1,'Informe o nome da categoria.').max(80,'Use até 80 caracteres.'),
  color:z.string().regex(/^#[0-9a-fA-F]{6}$/,'Escolha uma cor válida.'),
});
export type CategoryForm = z.infer<typeof categorySchema>;
