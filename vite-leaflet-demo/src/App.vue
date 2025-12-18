<template>
  <div id="map"></div>
</template>

<script setup>
import { onMounted } from 'vue'
import L from 'leaflet'

onMounted(() => {
  const map = L.map('map').setView([39.90923, 116.397428], 11)

  // 高德地图瓦片
  L.tileLayer(
    'https://webrd0{s}.is.autonavi.com/appmaptile?lang=zh_cn&size=1&scale=1&style=7&x={x}&y={y}&z={z}',
    {
      subdomains: ['1', '2', '3', '4'],
      maxZoom: 18,
      attribution: '© 高德地图'
    }
  ).addTo(map)
})
</script>

<style>
#map {
  width: 100vw;
  height: 100vh;
}
</style>
