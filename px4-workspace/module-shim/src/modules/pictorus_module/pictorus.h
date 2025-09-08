typedef struct AppInterface AppInterface;



#ifdef __cplusplus
extern "C" {
#endif // __cplusplus

AppInterface *app_interface_new(void);

void app_interface_free(struct AppInterface *app);

void app_interface_update(struct AppInterface *app, uint64_t app_time);

#ifdef __cplusplus
} // extern "C"
#endif // __cplusplus
