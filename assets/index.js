function refresh_metrics() {
   const cpu_usage_label = document.getElementById('cpu-usage-label');
   const mem_usage_label = document.getElementById('mem-usage-label');
   const swap_usage_label = document.getElementById('swap-usage-label');

   const uptime = document.getElementById('uptime');
   const cpu_usage = document.getElementById('cpu-usage');
   const mem_usage = document.getElementById('mem-usage');
   const swap_usage = document.getElementById('swap-usage');

   const current_system = document.getElementById('current-system');

   fetch('metrics')
      .then((response) => response.json())
      .then((data) => {
         uptime.textContent = format_duration(data.uptime.secs);

         let cpu = data.cpu_delta.used / data.cpu_delta.total;
         let mem = data.memory.used / data.memory.total;
         let swap = data.swap.used / data.swap.size;

         cpu_usage_label.textContent = `CPU ${Math.round(cpu*100)}%`;
         cpu_usage.value = cpu;

         mem_usage_label.textContent = `MEM ${Math.round(mem*100)}%`;
         mem_usage.value = mem;

         swap_usage_label.textContent = `SWP ${Math.round(swap*100)}%`;
         swap_usage.value = swap;

         current_system.textContent = data.current_system;
      })
      .catch((error) => {
         console.error('Error when refreshing metrics:', error);
      });
}

function format_duration(secs) {
   let days = Math.round(secs / 86400);
   let hours = Math.round((secs % 86400) / 3600);
   let mins = Math.round((secs % 3600) / 60);
   return `${days}d.${hours}h.${mins}m`;
}

function init_metrics(second_interval) {
   refresh_metrics();
   setInterval(refresh_metrics, second_interval * 1000);
}
