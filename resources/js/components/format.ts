/** "now", "4 min ago", then the clock time (local), as the design labels times. */
export function when(ms: number, now = Date.now()): string {
  const s = Math.round((now - ms) / 1000)
  if (s < 60) return 'now'
  if (s < 3600) return `${Math.round(s / 60)} min ago`
  return clock(ms)
}

export function clock(ms: number): string {
  return new Date(ms).toLocaleTimeString('en-GB', { hour: '2-digit', minute: '2-digit' })
}

export function firstName(name: string): string {
  return name.split(' ')[0]
}

const words = ['No', 'One', 'Two', 'Three', 'Four', 'Five', 'Six', 'Seven', 'Eight', 'Nine', 'Ten']
export function count(n: number): string {
  return words[n] ?? String(n)
}
