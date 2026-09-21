/** Match Turkish labels with either Turkish characters or a plain keyboard. */
export function searchText(value: string) {
  return value
    .toLocaleLowerCase("tr-TR")
    .normalize("NFD")
    .replace(/\p{M}/gu, "")
    .replace(/ı/g, "i")
    .trim();
}
