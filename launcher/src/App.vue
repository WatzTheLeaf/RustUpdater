<script setup lang="ts">
import { ref, onMounted } from 'vue';
import { invoke } from '@tauri-apps/api/core';
import { listen } from '@tauri-apps/api/event';

// Shadcn Components
import { Button } from '@/components/ui/button';
import { Input } from '@/components/ui/input';
import { Dialog, DialogContent, DialogDescription, DialogFooter, DialogHeader, DialogTitle } from '@/components/ui/dialog';
import { Select, SelectContent, SelectGroup, SelectItem, SelectTrigger, SelectValue } from '@/components/ui/select';
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from '@/components/ui/card';
import { ScrollArea } from '@/components/ui/scroll-area';
import { Progress } from '@/components/ui/progress';

// State
const showUrlModal = ref(true);
const serverUrlInput = ref('http://127.0.0.1:3000/');
const serverUrl = ref('');
const urlError = ref('');

const products = ref<Record<string, any>>({});
const selectedProductName = ref('');
const selectedProductData = ref<any>(null);
const localVersion = ref<string | null>(null);
const targetInstallVersion = ref('');

const isBusy = ref(false);
const currentTaskName = ref('');
const progressData = ref({ current: 0, total: 0, percent: 0 });

const showLogsModal = ref(false);
const logs = ref<string[]>([]);

onMounted(async () => {
  // Listen for logs emitted
  await listen<string>('log', (event) => {
    logs.value.push(event.payload);
  });

  // Listen for detailed progress emitted from Rust
  await listen<any>('progress', (event) => {
    progressData.value = event.payload;
  });
});

async function submitServerUrl() {
  try {
    urlError.value = '';
    serverUrl.value = await invoke('validate_server_url', { url: serverUrlInput.value });
    showUrlModal.value = false;
    await refreshData();
  } catch (err: any) {
    urlError.value = err;
  }
}

async function refreshData() {
  if (!serverUrl.value) return;
  try {
    const rootJson: any = await invoke('fetch_root', { serverUrl: serverUrl.value });
    products.value = rootJson.products || {};

    if (selectedProductName.value) {
      await selectProduct(selectedProductName.value);
    }
  } catch (err: any) {
    alert("Failed to fetch root.json: " + err);
  }
}

async function selectProduct(name: string) {
  selectedProductName.value = name;
  selectedProductData.value = products.value[name];

  targetInstallVersion.value = selectedProductData.value.latest_version;
  localVersion.value = await invoke('get_local_version', { productName: name });
}

async function updateProduct() {
  isBusy.value = true;
  currentTaskName.value = 'Downloading & Applying Updates';
  progressData.value = { current: 0, total: 0, percent: 0 }; // Reset progress
  logs.value.push(`--- Starting Update for ${selectedProductName.value} ---`);

  try {
    await invoke('run_update', {
      serverUrl: serverUrl.value,
      productName: selectedProductName.value,
      targetVersion: targetInstallVersion.value || selectedProductData.value.latest_version,
      availableVersions: selectedProductData.value.versions
    });
    progressData.value.percent = 100;
    await selectProduct(selectedProductName.value);
  } catch (err: any) {
    logs.value.push(`ERROR: ${err}`);
  } finally {
    setTimeout(() => { isBusy.value = false; }, 1000); // Give it a second to show 100%
  }
}

async function launchApp() {
  isBusy.value = true;
  currentTaskName.value = 'Launching App';
  progressData.value = { current: 0, total: 0, percent: 100 }; // Fake full bar for launch
  try {
    await invoke('launch_product', {
      serverUrl: serverUrl.value,
      productName: selectedProductName.value,
    });
  } catch (err: any) {
    alert(`Failed to launch: ${err}`);
  } finally {
    isBusy.value = false;
  }
}

async function verifyFiles() {
  if (!localVersion.value) return;
  isBusy.value = true;
  currentTaskName.value = 'Verifying Integrity';
  progressData.value = { current: 0, total: 0, percent: 0 }; // Reset progress
  logs.value.push(`--- Starting Integrity Check ---`);

  try {
    const corruptedFiles: string[] = await invoke('verify_integrity', {
      serverUrl: serverUrl.value,
      productName: selectedProductName.value,
      version: localVersion.value
    });

    if (corruptedFiles.length > 0) {
      logs.value.push(`Found ${corruptedFiles.length} corrupted files. Run an update to repair.`);
    }
    progressData.value.percent = 100;
  } catch (err: any) {
    logs.value.push(`ERROR: ${err}`);
  } finally {
    setTimeout(() => { isBusy.value = false; }, 1000);
  }
}
</script>

<template>
  <div class="flex h-screen w-screen bg-background text-foreground font-sans overflow-hidden">

    <aside class="w-64 bg-card p-4 flex flex-col border-r border-border">
      <h1 class="text-xl font-bold mb-6 text-primary">Launcher</h1>

      <div v-if="Object.keys(products).length === 0" class="text-muted-foreground text-sm">
        No products found.
      </div>

      <div class="flex flex-col gap-2 flex-1 overflow-y-auto">
        <Button
            v-for="(entry, name) in products"
            :key="name"
            :variant="selectedProductName === name ? 'default' : 'ghost'"
            class="justify-start w-full"
            @click="selectProduct(String(name))"
        >
          {{ name }}
        </Button>
      </div>

      <div class="mt-auto flex flex-col gap-2 pt-4 border-t border-border">
        <Button variant="secondary" @click="showLogsModal = true">
          View Logs
        </Button>
        <Button variant="outline" @click="refreshData">
          Refresh Data
        </Button>
      </div>
    </aside>

    <main class="flex-1 p-8 overflow-y-auto bg-muted/20">
      <Card v-if="selectedProductName" class="max-w-3xl mx-auto shadow-lg">
        <CardHeader>
          <CardTitle class="text-3xl">{{ selectedProductName }}</CardTitle>
          <CardDescription>Manage your installation and updates.</CardDescription>
        </CardHeader>

        <CardContent>
          <div class="flex gap-8 mb-6 text-sm">
            <div class="flex flex-col">
              <span class="text-muted-foreground">Local Version</span>
              <span class="font-mono font-medium text-lg">{{ localVersion || 'Not Installed' }}</span>
            </div>
            <div class="flex flex-col">
              <span class="text-muted-foreground">Latest Version</span>
              <span class="font-mono font-medium text-lg text-primary">{{ selectedProductData.latest_version }}</span>
            </div>
          </div>

          <div class="space-y-4 pt-6 border-t border-border">

            <div v-if="!localVersion" class="flex items-center gap-4">
              <Select v-model="targetInstallVersion">
                <SelectTrigger class="w-[180px]">
                  <SelectValue placeholder="Select version" />
                </SelectTrigger>
                <SelectContent>
                  <SelectGroup>
                    <SelectItem v-for="v in selectedProductData.versions" :key="v" :value="v">
                      Install v{{ v }}
                    </SelectItem>
                  </SelectGroup>
                </SelectContent>
              </Select>

              <Button @click="updateProduct" :disabled="isBusy" size="lg">
                Install Product
              </Button>
            </div>

            <div v-if="localVersion" class="space-y-6">

              <div v-if="localVersion !== selectedProductData.latest_version" class="bg-blue-900/20 border border-blue-800 p-4 rounded-lg flex items-center justify-between">
                <div>
                  <h4 class="font-bold text-blue-400">Update Available!</h4>
                  <p class="text-sm text-blue-200">Version {{ selectedProductData.latest_version }} is ready to install.</p>
                </div>
                <Button @click="updateProduct" :disabled="isBusy" class="bg-blue-600 hover:bg-blue-500 text-white">
                  Update Now
                </Button>
              </div>

              <div v-else class="text-green-500 font-bold flex items-center gap-2">
                <svg xmlns="http://www.w3.org/2000/svg" width="20" height="20" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M22 11.08V12a10 10 0 1 1-5.93-9.14"/><polyline points="22 4 12 14.01 9 11.01"/></svg>
                Product is fully up to date!
              </div>

              <div class="flex gap-4 w-full">
                <Button @click="launchApp" :disabled="isBusy" size="lg" class="flex-1 text-lg h-12">
                  Launch Product
                </Button>
                <Button variant="secondary" @click="verifyFiles" :disabled="isBusy" size="lg" class="h-12">
                  Verify
                </Button>
              </div>
            </div>

            <div v-if="isBusy" class="pt-6 space-y-2 animate-in fade-in slide-in-from-bottom-2">
              <div class="flex justify-between items-end">
                <span class="text-sm font-medium text-muted-foreground">
                  {{ currentTaskName }}...
                  <span v-if="progressData.total > 0">({{ progressData.current }} / {{ progressData.total }} files)</span>
                </span>
                <button @click="showLogsModal = true" class="text-xs text-primary hover:underline">View Logs</button>
              </div>
              <Progress :model-value="progressData.percent" class="h-2 w-full" :class="{'animate-pulse': progressData.percent === 0}" />
            </div>

          </div>
        </CardContent>
      </Card>

      <div v-else class="flex h-full items-center justify-center text-muted-foreground">
        Select a product from the menu to manage it.
      </div>
    </main>

    <Dialog :open="showUrlModal" @update:open="(val) => { if(!showUrlModal) showUrlModal = val }">
      <DialogContent class="sm:max-w-md" @pointer-down-outside.prevent @escape-key-down.prevent>
        <DialogHeader>
          <DialogTitle>Connect to Update Server</DialogTitle>
          <DialogDescription>
            Please provide the URL of your update server to continue.
          </DialogDescription>
        </DialogHeader>
        <div class="flex items-center space-x-2">
          <Input
              v-model="serverUrlInput"
              placeholder="http://127.0.0.1:3000/"
              @keyup.enter="submitServerUrl"
          />
        </div>
        <p v-if="urlError" class="text-destructive text-sm">{{ urlError }}</p>
        <DialogFooter class="sm:justify-end">
          <Button type="button" @click="submitServerUrl">
            Connect
          </Button>
        </DialogFooter>
      </DialogContent>
    </Dialog>

    <Dialog :open="showLogsModal" @update:open="showLogsModal = $event">
      <DialogContent class="max-w-3xl h-[70vh] flex flex-col">
        <DialogHeader>
          <DialogTitle>Process Logs</DialogTitle>
          <DialogDescription>Live output from the updater engine.</DialogDescription>
        </DialogHeader>

        <ScrollArea class="flex-1 w-full rounded-md border p-4 bg-black/90">
          <div class="font-mono text-sm text-green-400 space-y-1">
            <div v-for="(log, idx) in logs" :key="idx">{{ log }}</div>
            <div v-if="logs.length === 0" class="text-muted-foreground">Waiting for process...</div>
          </div>
        </ScrollArea>

        <DialogFooter>
          <Button variant="outline" @click="showLogsModal = false">Close</Button>
        </DialogFooter>
      </DialogContent>
    </Dialog>

  </div>
</template>