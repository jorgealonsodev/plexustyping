import { type CodeLanguage, codeLanguages } from "./api";

/** The bundled code languages, fetched once and shared by the header and the settings. */
class Languages {
  list = $state.raw<CodeLanguage[]>([]);
  private loading: Promise<void> | null = null;

  load(): Promise<void> {
    this.loading ??= codeLanguages()
      .then((list) => void (this.list = list))
      .catch((err) => {
        this.loading = null;
        console.warn("code_languages failed", err);
      });
    return this.loading;
  }

  name(id: string): string {
    return this.list.find((l) => l.id === id)?.name ?? id;
  }
}

export const LANGUAGES = new Languages();
