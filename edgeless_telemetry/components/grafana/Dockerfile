FROM grafana/grafana

# copy the custom config file
COPY custom.ini /etc/grafana/grafana.ini

# copy the files needed to provision a default dashboard and datasource
COPY default-datasource-provider.yaml /etc/grafana/provisioning/datasources/main.yaml
COPY default-dashboard-provider.yaml /etc/grafana/provisioning/dashboards/main.yaml
COPY default-dashboard.json /var/lib/grafana/dashboards/main-dashboard.json

EXPOSE 3000 

# we need to pass the config to the server command
CMD ["grafana-server", "--config=/etc/grafana/grafana.ini"]