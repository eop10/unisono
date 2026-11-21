//! Вызовы Tauri. Каталог инструментов и строёв — из ядра (get_catalog).

import { invoke } from '@tauri-apps/api/core';
import type { Catalog } from '../lib/types';

export async function getCatalog(): Promise<Catalog> {
  return await invoke<Catalog>('get_catalog');
}