// Configuração de verificação para ambientes que não permitem subprocessos do esbuild.
// As fontes são as mesmas do build normal; o TypeScript faz a transformação em processo.
import ts from 'typescript';
import { fileURLToPath } from 'node:url';
export function validationConfig({ production = false } = {}) {
  return {
    configFile: false,
    resolve: { preserveSymlinks: true, alias: { '@': fileURLToPath(new URL('../src', import.meta.url)) } },
    esbuild: false,
    plugins: [{
      name: 'typescript-in-process-validation', enforce: 'pre',
      transform(source, id) {
        const isTypeScript = /\.[cm]?tsx?(\?|$)/.test(id);
        if (!isTypeScript && !(production && /\.[cm]?jsx?(\?|$)/.test(id))) return null;
        const environment = (context) => {
          const isEnv = (node) => ts.isPropertyAccessExpression(node) && node.name.text === 'env' && ts.isIdentifier(node.expression) && node.expression.text === 'process';
          const visitor = (node) => {
            if (ts.isPropertyAccessExpression(node) && node.name.text === 'NODE_ENV' && isEnv(node.expression)) return context.factory.createStringLiteral('production');
            if (ts.isElementAccessExpression(node) && isEnv(node.expression) && ts.isStringLiteral(node.argumentExpression) && node.argumentExpression.text === 'NODE_ENV') return context.factory.createStringLiteral('production');
            if (isEnv(node)) return context.factory.createObjectLiteralExpression();
            return ts.visitEachChild(node, visitor, context);
          };
          return (source) => ts.visitNode(source, visitor);
        };
        const output = ts.transpileModule(source, { fileName: id.split('?')[0], compilerOptions: {
          target: ts.ScriptTarget.ES2022, module: ts.ModuleKind.ESNext, jsx: ts.JsxEmit.ReactJSX,
          sourceMap: true, inlineSources: true, esModuleInterop: true,
        }, transformers: production ? { before: [environment] } : undefined });
        return { code: output.outputText, map: output.sourceMapText ? JSON.parse(output.sourceMapText) : null };
      },
    }],
  };
}
