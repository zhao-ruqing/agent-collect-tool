import { ref } from 'vue'
import { defineStore } from 'pinia'

export const useFilterStore = defineStore('filter', () => {
  const toolType = ref('')

  function setToolType(val: string) {
    toolType.value = val
  }

  return { toolType, setToolType }
})
