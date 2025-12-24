<template>
  <div class="map-wrapper">
    <div id="map"></div>
    <div class="crosshair">
      <span class="h"></span>
      <span class="v"></span>
    </div>
    <a
        href="https://github.com/chainal/red-wolf-project"
        target="_blank"
        rel="noopener noreferrer"
        class="github-corner"
        aria-label="GitHub"
      >
        <!-- 官方 GitHub Mark SVG -->
        <svg height="32" width="32" viewBox="0 0 16 16" fill="currentColor" aria-hidden="true">
          <path
            fill-rule="evenodd"
            d="M8 0C3.58 0 0 3.58 0 8c0 3.54 2.29 6.53 5.47 7.59.4.07.55-.17.55-.38 
              0-.19-.01-.82-.01-1.49-2.01.37-2.53-.49-2.69-.94-.09-.23-.48-.94-.82-1.13-.28-.15-.68-.52-.01-.53.63-.01 
              1.08.58 1.23.82.72 1.21 1.87.87 2.33.66.07-.52.28-.87.51-1.07-1.78-.2-3.64-.89-3.64-3.95 
              0-.87.31-1.59.82-2.15-.08-.2-.36-1.01.08-2.1 0 0 .67-.21 2.2.82.64-.18 1.32-.27 2-.27.68 
              0 1.36.09 2 .27 1.53-1.04 2.2-.82 2.2-.82.44 1.09.16 1.9.08 2.1.51.56.82 1.27.82 2.15 
              0 3.07-1.87 3.75-3.65 3.95.29.25.54.73.54 1.48 0 1.07-.01 1.93-.01 2.2 0 .21.15.46.55.38A8.013 
              8.013 0 0 0 16 8c0-4.42-3.58-8-8-8z"
          />
        </svg>
    </a>
    <var-fab right="40" bottom="40" z-index="1002" v-model:active="fabActive">
      <var-button type="info" icon-container @click.stop="onClickUpload">
        <var-icon name="map-marker-radius" />
      </var-button>
    </var-fab>
    
    <div class="chip-layer">
      <var-chip>
          <var-icon name="account-circle" />
          {{ userNameChip }}
      </var-chip>
    </div>
    
  </div>
  
</template>

<script setup>
import { onMounted, ref } from 'vue'
import L from 'leaflet'

const MAP_VIEW_KEY = 'leaflet:lastView'
const USER_NAME_KEY = 'chip:username'
const fabActive = ref(false)
const userNameChip = ref('...')
let markerLayer = null

async function onClickUpload() {
  const { lat, lng, _ } = JSON.parse(localStorage.getItem(MAP_VIEW_KEY))
  const savedUsername = localStorage.getItem(USER_NAME_KEY)
  const payload = {
    user: savedUsername || null,
    lng: lng,
    lat: lat,
  }

  try {
    const resp = await fetch('/api/userposition', {
      method: 'POST',
      headers: {
        'Content-Type': 'application/json',
      },
      body: JSON.stringify(payload),
    })

    if (!resp.ok) {
      throw new Error(`HTTP ${resp.status}`)
    }

    /** @type {{ id: string, user: string }} */
    const data = await resp.json()

    console.log('Create Success:', JSON.stringify(data))
    localStorage.setItem(USER_NAME_KEY, data.user)
    userNameChip.value = data.user
    fabActive.value = false

    await loadPoints(lng, lat)

  } catch (err) {
    console.error('Create Error:', err)
  }
  
}

onMounted(() => {
  const savedUsername = localStorage.getItem(USER_NAME_KEY)
  if (savedUsername) {
    userNameChip.value = savedUsername
  }

  const map = L.map('map')

  // 高德地图瓦片
  L.tileLayer(
    'https://webrd0{s}.is.autonavi.com/appmaptile?lang=zh_cn&size=1&scale=1&style=7&x={x}&y={y}&z={z}',
    {
      subdomains: ['1', '2', '3', '4'],
      maxZoom: 18,
      attribution: '© 高德地图'
    }
  ).addTo(map)
  markerLayer = L.layerGroup().addTo(map)

  restoreView(map)

  saveMapView(map)

  if (L.Browser.touch) {
    enableDoubleTapZoom(map)
  }
})

function enableDoubleTapZoom(map) {
  let lastTap = 0;

  const container = map.getContainer();

  container.addEventListener('touchend', function (e) {
    const now = Date.now();
    const delta = now - lastTap;

    if (delta > 0 && delta < 300) {
        // 模拟双击放大
        map.zoomIn();
        e.preventDefault();
    }

    lastTap = now;
  }, { passive: false });
}

function restoreView(map) {
  const saved = localStorage.getItem(MAP_VIEW_KEY)
  if (saved) {
    try {
      const { lat, lng, zoom } = JSON.parse(saved)
      map.setView([lat, lng], zoom)
    } catch {
      map.setView([39.90923, 116.397428], 11)
    }
  } else {
    map.setView([39.90923, 116.397428], 11)
  }
}

function saveMapView(map) {
  const save = async () => {
    const center = map.getCenter()
    const zoom = map.getZoom()

    localStorage.setItem(
      MAP_VIEW_KEY,
      JSON.stringify({
        lat: center.lat,
        lng: center.lng,
        zoom
      })
    )

    await loadPoints(center.lng, center.lat)
  }

  // 移动 & 缩放结束时保存
  map.on('moveend zoomend', save)
}

async function loadPoints(lng, lat) {
  try {
    const params = new URLSearchParams({
      lng: String(lng),
      lat: String(lat),
    })
    const resp = await fetch('/api/userposition?' + params.toString())
    if (!resp.ok) {
      throw new Error('HTTP ' + resp.status)
    }
    const points = await resp.json()
    renderPoints(points)
  } catch (err) {
    console.error('fetch points error', err)
  }

}

function renderPoints(points) {
  markerLayer.clearLayers()

  points.forEach(p => {
    const lng = p.location[0]
    const lat = p.location[1]

    // 外圈（反色 / 深色）
  L.circleMarker([lat, lng], {
    radius: 6,
    color: '#ffffff',     // 描边颜色（白色 / 反色）
    weight: 2,
    fillOpacity: 1,
    fillColor: '#e53935', // 可选：给外圈一个深色填充
  }).addTo(markerLayer)

  // 内圈（主点）
  L.circleMarker([lat, lng], {
    radius: 4,
    color: '#e53935',
    weight: 1,
    fillColor: '#e53935',
    fillOpacity: 0.9,
  })
      .bindPopup(p.user + '<br>' + p.createTime)
      .addTo(markerLayer)
  })
}

</script>

<style>
.map-wrapper {
  position: relative;
  width: 100vw;
  height: 100vh;
}

#map {
  width: 100%;
  height: 100%;
}

/* 准星本体 */
.crosshair {
  position: absolute;
  left: 50%;
  top: 50%;
  width: 25px;
  height: 25px;
  transform: translate(-50%, -50%);
  pointer-events: none; /* 不挡地图操作 */
  z-index: 1000;
}

/* 横线 */
.crosshair .h {
  position: absolute;
  top: 50%;
  left: 0;
  width: 100%;
  height: 2px;
  background: #ff3b30;
  transform: translateY(-50%);
}

/* 竖线 */
.crosshair .v {
  position: absolute;
  left: 50%;
  top: 0;
  width: 2px;
  height: 100%;
  background: #ff3b30;
  transform: translateX(-50%);
}

.github-corner {
  position: fixed;
  top: 10px;
  right: 10px;
  color: #333;
  text-decoration: none;
  cursor: pointer;
  transition: color 0.2s ease-in-out;
  z-index: 1001;
}

.github-corner:hover {
  color: #4078c0;
}

.chip-layer {
  position: absolute;
  left: 20px;
  bottom: 20px;
  z-index: 1003;
}
</style>
