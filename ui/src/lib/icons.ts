import type { Component } from 'vue'
import {
  Banknote, Building, Car, Circle, CircleEllipsis, Clapperboard, Coins, CreditCard, Gift, GraduationCap,
  HandCoins, HandHeart, HeartPulse, Landmark, PiggyBank, Receipt, ShoppingBag, Smartphone, TrendingUp, Users,
  Utensils, Wallet,
} from '@lucide/vue'

/** Nama ikon (lucide, kebab-case) yang disimpan di DB → komponen. */
export const ICONS: Record<string, Component> = {
  utensils: Utensils,
  car: Car,
  receipt: Receipt,
  'shopping-bag': ShoppingBag,
  'heart-pulse': HeartPulse,
  clapperboard: Clapperboard,
  'graduation-cap': GraduationCap,
  users: Users,
  landmark: Landmark,
  'circle-ellipsis': CircleEllipsis,
  wallet: Wallet,
  gift: Gift,
  'trending-up': TrendingUp,
  'hand-heart': HandHeart,
  banknote: Banknote,
  'credit-card': CreditCard,
  'piggy-bank': PiggyBank,
  smartphone: Smartphone,
  // @lucide/vue v1 tidak punya Building2; nama tetap agar data tidak perlu migrasi.
  'building-2': Building,
  coins: Coins,
  'hand-coins': HandCoins,
}

export function iconFor(name: string | null): Component {
  return (name && ICONS[name]) || Circle
}
