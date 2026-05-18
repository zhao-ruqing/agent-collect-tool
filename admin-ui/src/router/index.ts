import { createRouter, createWebHistory } from 'vue-router'
import MainLayout from '../layouts/MainLayout.vue'

const router = createRouter({
  history: createWebHistory(import.meta.env.BASE_URL),
  routes: [
    {
      path: '/',
      component: MainLayout,
      redirect: '/dashboard',
      children: [
        {
          path: 'dashboard',
          name: 'dashboard',
          component: () => import('../views/Dashboard.vue'),
        },
        {
          path: 'conversations',
          name: 'conversations',
          component: () => import('../views/Conversations.vue'),
        },
        {
          path: 'edits',
          name: 'edits',
          component: () => import('../views/CodeEdits.vue'),
        },
        {
          path: 'events',
          name: 'events',
          component: () => import('../views/Events.vue'),
        },
        {
          path: 'agents',
          name: 'agents',
          component: () => import('../views/Agents.vue'),
        },
      ],
    },
  ],
})

export default router
