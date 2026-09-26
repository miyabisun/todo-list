export interface Todo {
  id: string;
  title: string;
  url: string | null;
  due: string | null;
  source: string | null;
  created_at: string;
  updated_at: string;
  done_at: string | null;
  body: string;
}

export interface NewTodoForm {
  title: string;
  url: string;
  due: string;
  body: string;
}

function parts(date: string): [number, number, number] {
  const [year, month, day] = date.split("-").map(Number);
  return [year, month, day];
}

function short(date: string, today: string): string {
  const [year, month, day] = parts(date);
  return year === parts(today)[0]
    ? `${month}/${day}`
    : `${year}/${month}/${day}`;
}

/** YYYY-MM-DD of the given moment in the viewer's time zone. */
export function localDate(date: Date): string {
  const pad = (value: number) => String(value).padStart(2, "0");
  return `${date.getFullYear()}-${pad(date.getMonth() + 1)}-${pad(date.getDate())}`;
}

/** Due-date text; overdue items say so in words as well as color. */
export function dueLabel(
  due: string,
  today: string,
): { text: string; overdue: boolean } {
  if (due < today) {
    return { text: `期限切れ ${short(due, today)}`, overdue: true };
  }
  const [year, month, day] = parts(today);
  if (due === today) {
    return { text: "今日まで", overdue: false };
  }
  if (due === localDate(new Date(year, month - 1, day + 1))) {
    return { text: "明日まで", overdue: false };
  }
  return { text: `${short(due, today)}まで`, overdue: false };
}

/** Links are rendered only for http(s); anything else stays plain text. */
export function safeUrl(url: string | null): string | null {
  return url && /^https?:\/\//i.test(url) ? url : null;
}

/** Request body for a new item: trimmed title, blank optional fields left out. */
export function newTodoFields(form: NewTodoForm): Record<string, string> {
  const fields: Record<string, string> = { title: form.title.trim() };
  const url = form.url.trim();
  if (url) fields.url = url;
  if (form.due) fields.due = form.due;
  if (form.body.trim()) fields.body = form.body;
  return fields;
}
