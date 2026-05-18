<template>
  <div class="dashboard">
    <h1>Agent Collect Tool Dashboard</h1>
    <el-row :gutter="20">
      <el-col :span="6">
        <el-card shadow="hover">
          <template #header>
            <div class="card-header">
              <span>Total Agents</span>
            </div>
          </template>
          <div class="card-body">
            <span class="count">1</span>
          </div>
        </el-card>
      </el-col>
      <el-col :span="6">
        <el-card shadow="hover">
          <template #header>
            <div class="card-header">
              <span>Total Sessions</span>
            </div>
          </template>
          <div class="card-body">
            <span class="count">10</span>
          </div>
        </el-card>
      </el-col>
    </el-row>
  </div>
</template>

<style scoped>
.dashboard {
  padding: 20px;
}
.card-header {
  display: flex;
  justify-content: space-between;
  align-items: center;
}
.card-body {
  font-size: 24px;
  font-weight: bold;
  text-align: center;
}
</style>
